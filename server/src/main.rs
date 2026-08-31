use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response, Json},
    routing::{get, delete, put, post},
    Router,
};
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::pin::Pin;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn, error};
use types::*;
use uuid::Uuid;
use reqwest::Client;
use anyhow::anyhow;
use futures::Future;

mod news_service;
mod types;
mod cache;
mod config;
mod user_source_manager;
mod ai_config;
mod ai_client;
mod web_scraper;

use news_service::NewsService;
use user_source_manager::UserSourceManager;
use ai_client::AIClient;
use web_scraper::WebScraper;



#[derive(Clone)]
struct AppState {
    news_service: Arc<NewsService>,
    cache: Arc<DashMap<String, CachedResponse>>,
    config: Arc<config::Config>,
    user_source_manager: Arc<Mutex<UserSourceManager>>,
}

// 缓存结构
#[derive(Clone)]
struct CachedResponse {
    data: serde_json::Value,
    timestamp: Instant,
    ttl: Duration,
}

impl CachedResponse {
    fn new(data: serde_json::Value, ttl: Duration) -> Self {
        Self {
            data,
            timestamp: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.timestamp.elapsed() > self.ttl
    }
}

// API响应结构
#[derive(Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
    message: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            message: None,
        }
    }
}

// 查询参数
#[derive(Deserialize)]
struct NewsQuery {
    no_cache: Option<bool>,
}

#[derive(Deserialize)]
struct CombinedNewsQuery {
    sources: Option<String>,  // 用逗号分隔的新闻源，如 "baidu,zhihu,weibo"
    no_cache: Option<bool>,
}

#[derive(Deserialize)]
struct ImageQuery {
    url: String,
}
// 
async fn health_check() -> impl IntoResponse {
    Json(ApiResponse::success("OK"))
}

async fn get_ai_providers() -> Json<ApiResponse<serde_json::Value>> {
    match ai_config::AIConfig::load().await {
        Ok(config) => {
            // Load the full config file to get providers
            match std::fs::read_to_string("config/ai_config.json") {
                Ok(content) => {
                    match serde_json::from_str::<serde_json::Value>(&content) {
                        Ok(full_config) => {
                            if let Some(providers) = full_config.get("ai_providers") {
                                Json(ApiResponse::success(providers.clone()))
                            } else {
                                Json(ApiResponse::error("AI providers not found in config".to_string()))
                            }
                        }
                        Err(e) => Json(ApiResponse::error(format!("Failed to parse config: {}", e)))
                    }
                }
                Err(e) => Json(ApiResponse::error(format!("Failed to read config file: {}", e)))
            }
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to load AI providers: {}", e))),
    }
}

// AI
async fn get_ai_config() -> Json<ApiResponse<serde_json::Value>> {
    match ai_config::AIConfig::load().await {
        Ok(config) => Json(ApiResponse::success(serde_json::to_value(config).unwrap())),
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!("Failed to load AI config: {}", e))),
    }
}

async fn update_ai_config(
    Json(config): Json<serde_json::Value>
) -> Json<ApiResponse<String>> {
    // Wrap the config in current_config structure
    let wrapped_config = serde_json::json!({
        "current_config": config
    });
    
    // Save the updated configuration
    let config_path = "config/ai_config.json";
    
    // 确保 config 目录存在
    if let Some(parent) = std::path::Path::new(config_path).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Json(ApiResponse::error(format!("Failed to create config directory: {}", e)));
        }
    }
    
    let content = match serde_json::to_string_pretty(&wrapped_config) {
        Ok(content) => content,
        Err(e) => return Json(ApiResponse::error(format!("Failed to serialize config: {}", e))),
    };
    
    match std::fs::write(config_path, content) {
        Ok(_) => Json(ApiResponse::success("AI configuration updated successfully".to_string())),
        Err(e) => Json(ApiResponse::error(format!("Failed to save config: {}", e))),
    }
}

async fn test_ai_connection() -> Json<ApiResponse<serde_json::Value>> {
    use crate::ai_client::AIClient;
    
    // Try to create AI client and test connection
    match AIClient::new().await {
        Ok(ai_client) => {
            // Test with a simple request
            let test_prompt = "Respond with 'OK' if you can read this message.";
            match ai_client.test_connection(test_prompt).await {
                Ok(response) => {
                    Json(ApiResponse::success(serde_json::json!({
                        "success": true,
                        "message": "AI connection test successful",
                        "response": response,
                        "provider": ai_client.get_provider_name(),
                        "model": ai_client.get_model_name()
                    })))
                }
                Err(e) => {
                    Json(ApiResponse::error(format!("AI connection test failed: {}", e)))
                }
            }
        }
        Err(e) => {
            Json(ApiResponse::error(format!("Failed to initialize AI client: {}", e)))
        }
    }
}

// User Sources
async fn get_user_sources(State(state): State<AppState>) -> impl IntoResponse {
    let user_manager = state.user_source_manager.lock().unwrap();
    let sources = user_manager.get_sources();
    Json(ApiResponse::success(sources))
}

async fn update_user_sources(
    Json(config): Json<serde_json::Value>
) -> Json<ApiResponse<String>> {
    // Save the updated configuration
    let config_path = "data/user_sources.json";
    let content = match serde_json::to_string_pretty(&config) {
        Ok(content) => content,
        Err(e) => return Json(ApiResponse::error(format!("Failed to serialize user sources: {}", e))),
    };
    
    // Ensure directory exists
    if let Some(parent) = std::path::Path::new(config_path).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Json(ApiResponse::error(format!("Failed to create directory: {}", e)));
        }
    }
    
    match std::fs::write(config_path, content) {
        Ok(_) => Json(ApiResponse::success("User sources updated successfully".to_string())),
        Err(e) => Json(ApiResponse::error(format!("Failed to save user sources: {}", e))),
    }
}


// Helper function to generate parsing rules for a source
async fn generate_parsing_rules_for_source(source: &mut types::UserNewsSource) -> Result<(), anyhow::Error> {
    let ai_client = AIClient::new().await?;
    let scraper = WebScraper::new();
    
    info!("Starting intelligent rule generation for source: {} ({})", source.name, source.url);
    
    // Step 1: Use selector field if available, otherwise fetch HTML
    // For rule generation, we want to analyze the selector snippet if provided
    let html_content = if let Some(selector_snippet) = &source.selector {
        info!("Using selector snippet ({} bytes) instead of fetching full HTML", selector_snippet.len());
        selector_snippet.clone()
    } else {
        let content = scraper.fetch_html_direct(&source.url, true).await?;
        info!("Fetched {} bytes of HTML content from {}", content.len(), source.url);
        content
    };
    
    // Step 2: Analyze content type and structure
    let content_type = detect_content_type(&html_content);
    info!("Detected content type: {}", content_type);
    
    // Step 3: Try to generate structured extraction rules first (preferred approach)
    match ai_client.generate_structured_extraction_rules(&source.url, &html_content).await {
        Ok(mut structured_rules) => {
            info!("Successfully generated structured extraction rules for source: {}", source.name);
            
            // Step 4: Validate and enhance the generated rules
            if let Err(validation_err) = validate_and_enhance_rules(&mut structured_rules, &html_content).await {
                warn!("Rule validation failed for {}: {}, attempting to fix", source.name, validation_err);
                // Try to fix common issues
                enhance_rules_based_on_content(&mut structured_rules, &html_content, &content_type);
            }
            
            source.parsing_rules = Some(types::ParsingRulesVariant::Structured(structured_rules));
            Ok(())
        }
        Err(e) => {
            warn!("Failed to generate structured extraction rules for source {}: {}, falling back to legacy rules", source.name, e);
            
            // Fallback to legacy rules
            let rules = ai_client.generate_parsing_rules(&source.url, &html_content).await?;
            source.parsing_rules = Some(types::ParsingRulesVariant::Legacy(rules));
            Ok(())
        }
    }
}

// Helper function to detect content type
fn detect_content_type(html_content: &str) -> String {
    let content_lower = html_content.to_lowercase();
    
    // Check for actual RSS/XML content first
    if content_lower.contains("<rss") || content_lower.contains("<feed") || 
       (content_lower.contains("<?xml") && (content_lower.contains("<channel>") || content_lower.contains("<entry>"))) {
        "RSS/Atom Feed".to_string()
    } else if content_lower.contains("<article") || content_lower.contains("class=\"article") || content_lower.contains("class=\"post\"") {
        "Article/Blog".to_string()
    } else if content_lower.contains("<table") && content_lower.contains("<tr>") && content_lower.contains("<td>") {
        "Table-based Layout".to_string()
    } else if content_lower.contains("<ul") && content_lower.contains("<li>") && (content_lower.contains("href") || content_lower.contains("news")) {
        "List/News".to_string()
    } else if content_lower.contains("github") && content_lower.contains("repository") {
        "GitHub".to_string()
    } else if content_lower.contains("<card") || content_lower.contains("class=\"card") {
        "Card-based Layout".to_string()
    } else {
        "General Web".to_string()
    }
}

// Helper function to validate and enhance rules
async fn validate_and_enhance_rules(
    rules: &mut types::ExtractionRules,
    html_content: &str
) -> Result<(), String> {
    // Basic validation - check if selectors exist in HTML
    if !html_content.contains(&rules.selectors.item_list) {
        return Err(format!("Item list selector '{}' not found in content", rules.selectors.item_list));
    }
    
    // Check if we have required fields
    if rules.selectors.fields.is_empty() {
        return Err("No field selectors defined".to_string());
    }
    
    // Validate essential fields exist
    let essential_fields = ["title", "url"];
    for field in essential_fields.iter() {
        if !rules.selectors.fields.contains_key(*field) {
            return Err(format!("Missing essential field: {}", field));
        }
    }
    
    Ok(())
}

// Helper function to enhance rules based on content analysis
fn enhance_rules_based_on_content(
    rules: &mut types::ExtractionRules,
    html_content: &str,
    content_type: &str
) {
    // Add common enhancements based on content type
    match content_type {
        "RSS/Atom Feed" => {
            // Ensure RSS-specific fields
            rules.selectors.fields.entry("title".to_string()).or_insert_with(|| types::FieldRule {
                selector: "title".to_string(),
                attribute: "text".to_string(),
                required: true,
                base_url: None,
                format: None,
                clean: true,
            });
            
            rules.selectors.fields.entry("url".to_string()).or_insert_with(|| types::FieldRule {
                selector: "link".to_string(),
                attribute: "text".to_string(),
                required: true,
                base_url: None,
                format: Some("url".to_string()),
                clean: true,
            });
            
            rules.selectors.fields.entry("timestamp".to_string()).or_insert_with(|| types::FieldRule {
                selector: "pubDate".to_string(),
                attribute: "text".to_string(),
                required: false,
                base_url: None,
                format: Some("datetime".to_string()),
                clean: true,
            });
        }
        "Article/Blog" => {
            // Blog-specific enhancements
            if !rules.selectors.fields.contains_key("author") && html_content.contains("author") {
                rules.selectors.fields.insert("author".to_string(), types::FieldRule {
                    selector: ".author, [class*='author'], .byline".to_string(),
                    attribute: "text".to_string(),
                    required: false,
                    base_url: None,
                    format: None,
                    clean: true,
                });
            }
        }
        _ => {
            // General web enhancements
            if !rules.selectors.fields.contains_key("description") && html_content.contains("desc") {
                rules.selectors.fields.insert("description".to_string(), types::FieldRule {
                    selector: ".description, .desc, .summary".to_string(),
                    attribute: "text".to_string(),
                    required: false,
                    base_url: None,
                    format: None,
                    clean: true,
                });
            }
        }
    }
}

// Generate parsing rules for user source
async fn generate_parsing_rules(
    Json(request): Json<serde_json::Value>
) -> Json<ApiResponse<serde_json::Value>> {
    let url: String = match request.get("url").and_then(|v| v.as_str()) {
        Some(url) => url.to_string(),
        None => return Json(ApiResponse::error("Missing url field".to_string())),
    };
    
    let source_name: String = match request.get("name").and_then(|v| v.as_str()) {
        Some(name) => name.to_string(),
        None => return Json(ApiResponse::error("Missing name field".to_string())),
    };
    
    info!("Generating parsing rules for source: {} ({})", source_name, url);
    
    // Initialize AI client and scraper
    let ai_client = match AIClient::new().await {
        Ok(client) => client,
        Err(e) => return Json(ApiResponse::error(format!("Failed to initialize AI client: {}", e))),
    };
    
    let scraper = WebScraper::new();
    
    // Fetch HTML content
    let html_content = match scraper.fetch_html(&url, true).await {
        Ok(content) => content,
        Err(e) => return Json(ApiResponse::error(format!("Failed to fetch web content: {}", e))),
    };
    
    // Generate parsing rules using AI
    let rules: types::ParsingRules = match ai_client.generate_parsing_rules(&url, &html_content).await {
        Ok(rules) => rules,
        Err(e) => return Json(ApiResponse::error(format!("Failed to generate parsing rules: {}", e))),
    };
    
    info!("Successfully generated parsing rules for source: {}", source_name);
    
    Json(ApiResponse::success(serde_json::to_value(rules).unwrap()))
}

// Generate structured extraction rules for user source
async fn generate_structured_extraction_rules(
    Json(request): Json<serde_json::Value>
) -> Json<ApiResponse<serde_json::Value>> {
    let url: String = match request.get("url").and_then(|v| v.as_str()) {
        Some(url) => url.to_string(),
        None => return Json(ApiResponse::error("Missing url field".to_string())),
    };
    
    let source_name: String = match request.get("name").and_then(|v| v.as_str()) {
        Some(name) => name.to_string(),
        None => return Json(ApiResponse::error("Missing name field".to_string())),
    };
    
    let selector: Option<String> = request.get("selector").and_then(|v| v.as_str()).map(|s| s.to_string());
    
    info!("Generating structured extraction rules for source: {} ({})", source_name, url);
    
    // Initialize AI client and scraper
    let ai_client = match AIClient::new().await {
        Ok(client) => client,
        Err(e) => return Json(ApiResponse::error(format!("Failed to initialize AI client: {}", e))),
    };
    
    // Use selector if provided, otherwise fetch HTML
    let html_content = if let Some(selector_snippet) = selector {
        info!("Using provided selector snippet ({} bytes) instead of fetching HTML", selector_snippet.len());
        selector_snippet
    } else {
        let scraper = WebScraper::new();
        match scraper.fetch_html(&url, true).await {
            Ok(content) => {
                info!("Fetched {} bytes of HTML content from {}", content.len(), url);
                content
            },
            Err(e) => return Json(ApiResponse::error(format!("Failed to fetch HTML content: {}", e))),
        }
    };
    
    // Generate structured extraction rules using AI
    let rules = match ai_client.generate_structured_extraction_rules(&url, &html_content).await {
        Ok(rules) => rules,
        Err(e) => return Json(ApiResponse::error(format!("Failed to generate structured extraction rules: {}", e))),
    };
    
    info!("Successfully generated structured extraction rules for source: {}", source_name);
    Json(ApiResponse::success(serde_json::to_value(rules).unwrap()))
}

// Extract relevant HTML region from full HTML using selector snippet
// The selector snippet contains an HTML example that helps identify the correct region
fn extract_html_region(full_html: &str, selector_snippet: &str) -> Option<String> {
    use scraper::{Html, Selector};
    
    let document = Html::parse_document(full_html);
    
    // Extract key features from selector snippet
    // Look for the first HTML tag and extract its class/id attributes
    let features = extract_selector_features(selector_snippet);
    
    if features.is_empty() {
        info!("No selector features found in snippet");
        return None;
    }
    
    info!("Extracted selector features: {:?}", features);
    
    // Try each feature combination to find the region
    for selector_str in features {
        if let Ok(selector) = Selector::parse(&selector_str) {
            if let Some(element) = document.select(&selector).next() {
                info!("Found HTML region using selector: {}", selector_str);
                return Some(element.html());
            }
        }
    }
    
    info!("No matching HTML region found for selector features");
    None
}

// Extract CSS selector features from HTML snippet
fn extract_selector_features(html_snippet: &str) -> Vec<String> {
    let mut features = Vec::new();
    
    // Parse the first tag from snippet
    if let Some(tag_start) = html_snippet.find('<') {
        if let Some(tag_end) = html_snippet[tag_start..].find('>') {
            let tag_content = &html_snippet[tag_start + 1..tag_start + tag_end];
            
            // Extract tag name (first word)
            let tag_name = tag_content.split_whitespace().next().unwrap_or("div");
            
            // Extract class attribute
            let class_value = extract_attribute(tag_content, "class");
            // Extract id attribute  
            let id_value = extract_attribute(tag_content, "id");
            
            // Build selectors in order of specificity
            // 1. tag with id (most specific)
            if let Some(id) = &id_value {
                features.push(format!("{}#{}", tag_name, id));
            }
            
            // 2. tag with class and id
            if let (Some(class), Some(id)) = (&class_value, &id_value) {
                features.push(format!("{}.{}#{}", tag_name, class.split_whitespace().next().unwrap_or(""), id));
            }
            
            // 3. tag with class
            if let Some(class) = &class_value {
                let first_class = class.split_whitespace().next().unwrap_or("");
                features.push(format!("{}.{}", tag_name, first_class));
                
                // Also try with all classes
                let all_classes: Vec<&str> = class.split_whitespace().collect();
                if all_classes.len() > 1 {
                    features.push(format!("{}{}", tag_name, all_classes.iter().map(|c| format!(".{}", c)).collect::<String>()));
                }
            }
            
            // 4. Just the tag (least specific, added last as fallback)
            if id_value.is_none() && class_value.is_none() {
                features.push(tag_name.to_string());
            }
        }
    }
    
    features
}

// Extract attribute value from HTML tag string
fn extract_attribute(tag_content: &str, attr_name: &str) -> Option<String> {
    let patterns = [
        format!(r#"{}="([^"]*)""#, attr_name),
        format!(r"{}='([^']*)'", attr_name),
        format!(r"{}=([^\s>]+)", attr_name),
    ];
    
    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(caps) = re.captures(tag_content) {
                if let Some(m) = caps.get(1) {
                    return Some(m.as_str().to_string());
                }
            }
        }
    }
    
    None
}

// Parse using stored rules (fast path)
async fn fetch_user_source_with_rules(
    source_name: &str, 
    user_source: &types::UserNewsSource
) -> Result<NewsSource, anyhow::Error> {
    info!("Using stored parsing rules for user data source: {}", source_name);
    
    let scraper = WebScraper::new();
    
    // Fetch HTML content directly (not RSS) for rule-based parsing
    let full_html = scraper.fetch_html_direct(&user_source.url, true).await
        .map_err(|e| anyhow!("Failed to fetch web content: {}", e))?;

            // info!("full_html:{}",full_html);
    
    // Extract the relevant region from HTML using selector field
    // The selector field contains an HTML snippet that helps identify the correct region
    let html_content = if let Some(selector_snippet) = &user_source.selector {
        // Extract key features from selector snippet (e.g., class, id attributes)
        let region_html = extract_html_region(&full_html, selector_snippet)
            .unwrap_or_else(|| {
                info!("Could not extract region using selector, using full HTML");
                full_html.clone()
            });

            // info!("region_html:{}",region_html);
        
        if region_html != full_html {
            info!("Successfully extracted HTML region ({} bytes) from full HTML ({} bytes)", 
                  region_html.len(), full_html.len());
        }
        region_html
    } else {
        full_html
    };
    
    // Parse using rules based on variant
    let news_items = match user_source.parsing_rules.as_ref()
        .ok_or_else(|| anyhow!("No parsing rules available for source: {}", source_name))? {
        types::ParsingRulesVariant::Legacy(rules) => {
            scraper.parse_news_with_rules(&html_content, rules).await
                .map_err(|e| anyhow!("Rules parsing failed: {}", e))?
        }
        types::ParsingRulesVariant::Structured(rules) => {
            scraper.parse_news_with_structured_rules(&html_content, rules).await
                .map_err(|e| anyhow!("Structured rules parsing failed: {}", e))?
        }
    };
    
    info!("Rules parsing successful, got {} news items", news_items.len());
    
    Ok(NewsSource {
        name: source_name.to_string(),
        title: user_source.title.clone(),
        description: user_source.description.clone(),
        link: user_source.url.clone(),
        items: news_items.clone(),
        total: news_items.len(),
        from_cache: false,
        update_time: chrono::Utc::now().to_rfc3339(),
        expires_at: None,
        ttl_minutes: None,
    })
}

// Regenerate parsing rules when rules parsing fails
async fn regenerate_parsing_rules(
    source_name: &str,
    user_source: &mut types::UserNewsSource
) -> Result<(), anyhow::Error> {
    info!("Regenerating parsing rules for source: {}", source_name);
    
    let ai_client = AIClient::new().await
        .map_err(|e| anyhow!("Failed to initialize AI client: {}", e))?;
    
    let scraper = WebScraper::new();
    
    // Use selector field if available, otherwise fetch HTML
    let html_content = if let Some(selector_snippet) = &user_source.selector {
        info!("Using selector snippet ({} bytes) instead of fetching full HTML", selector_snippet.len());
        selector_snippet.clone()
    } else {
        let content = scraper.fetch_html_direct(&user_source.url, true).await
            .map_err(|e| anyhow!("Failed to fetch web content: {}", e))?;
        info!("Fetched {} bytes of HTML content from {}", content.len(), user_source.url);
        content
    };
    
    // Generate new structured rules using AI
    let mut new_rules = ai_client.generate_structured_extraction_rules(&user_source.url, &html_content).await
        .map_err(|e| anyhow!("Failed to generate new parsing rules: {}", e))?;
    
    // Update statistics
    new_rules.total_attempts = match user_source.parsing_rules.as_ref() {
        Some(types::ParsingRulesVariant::Legacy(rules)) => rules.total_attempts + 1,
        Some(types::ParsingRulesVariant::Structured(rules)) => rules.total_attempts + 1,
        None => 1,
    };
    
    user_source.parsing_rules = Some(types::ParsingRulesVariant::Structured(new_rules));
    
    info!("Successfully regenerated parsing rules for source: {}", source_name);
    Ok(())
}

// RustAI
async fn fetch_user_source_with_ai(source_name: &str, source_url: &str) -> Result<NewsSource, anyhow::Error> {
    info!("Starting Rust direct AI parsing for user data source: {}", source_name);
    
    // AI
    let ai_client = AIClient::new().await
        .map_err(|e| anyhow!("Failed to initialize AI client: {}", e))?;
    let scraper = WebScraper::new();
    
    // 
    let user_source = {
        let config = types::UserSourcesConfig::load().await
            .map_err(|e| anyhow!("Failed to load user sources: {}", e))?;
        config.find_source_by_name(source_name)
            .ok_or_else(|| anyhow!("User data source not found: {}", source_name))?
            .clone()
    };
    
    let news_items = match user_source.source_type {
        types::UserSourceType::Json => {
            // For JSON API sources, fetch and parse JSON directly
            info!("Fetching JSON API content from: {}", source_url);
            let json_content = scraper.fetch_html_direct(source_url, false).await
                .map_err(|e| anyhow!("Failed to fetch JSON content: {}", e))?;
            info!("Fetched JSON content (first 500 chars): {}", &json_content[..json_content.len().min(500)]);
            
            // Generate field mapping rules if not available
            let field_mapping_rules = if user_source.field_mapping_rules.is_none() {
                info!("Generating field mapping rules for new JSON API source: {}", source_name);
                match ai_client.generate_field_mapping_rules(&json_content, source_url).await {
                    Ok(rules) => {
                        // Save the generated rules to the user source
                        let mut config = types::UserSourcesConfig::load().await
                            .map_err(|e| anyhow!("Failed to load user sources: {}", e))?;
                        if let Some(source) = config.user_sources.iter_mut().find(|s| s.name == source_name) {
                            source.field_mapping_rules = Some(rules.clone());
                            if let Err(save_err) = config.save() {
                                warn!("Failed to save field mapping rules: {}", save_err);
                            }
                        }
                        Some(rules)
                    }
                    Err(e) => {
                        warn!("Failed to generate field mapping rules: {}, using fallback", e);
                        None
                    }
                }
            } else {
                user_source.field_mapping_rules.clone()
            };
            
            ai_client.parse_news_from_json(source_url, &json_content, field_mapping_rules.as_ref()).await
                .map_err(|e| anyhow!("AI JSON parsing failed: {}", e))?
        }
        types::UserSourceType::Web => {
            // For web sources, fetch HTML and parse
            info!("Fetching HTML content from: {}", source_url);
            let html_content = scraper.fetch_html(source_url, true).await
                .map_err(|e| anyhow!("Failed to fetch web content: {}", e))?;
            
            let selector = user_source.selector.as_deref();
            ai_client.parse_news_from_html(source_url, &html_content, selector).await
                .map_err(|e| anyhow!("AI HTML parsing failed: {}", e))?
        }
        _ => {
            return Err(anyhow!("Unsupported source type: {}", user_source.source_type));
        }
    };
    
    info!("AI parsing successful, got {} news items", news_items.len());
    
    // 
    Ok(NewsSource {
        name: source_name.to_string(),
        title: user_source.title,
        description: user_source.description,
        link: source_url.to_string(),
        items: news_items.clone(),
        total: news_items.len(),
        from_cache: false,
        update_time: chrono::Utc::now().to_rfc3339(),
        expires_at: None,
        ttl_minutes: None,
    })
}

// 
async fn get_news_simple(
    State(state): State<AppState>,
    Path(source): Path<String>,
    Query(query): Query<NewsQuery>,
) -> Json<ApiResponse<NewsSource>> {
    let no_cache = query.no_cache.unwrap_or(false);
    
    // Load existing config
    let config: types::UserSourcesConfig = match types::UserSourcesConfig::load().await {
        Ok(config) => config,
        Err(e) => {
            // If file doesn't exist, create new config
            if e.kind() == std::io::ErrorKind::NotFound {
                types::UserSourcesConfig::new()
            } else {
                return Json(ApiResponse::<NewsSource>::error(format!("Failed to load user sources: {}", e)));
            }
        }
    };
    
    // Check if it's a user source
    if let Some(user_source) = config.clone().find_source_by_name(&source) {
        // It's a user source, try intelligent parsing strategy
        info!("Detected user source: {}, checking for parsing rules", source);
        
        // Strategy 1: Try using stored rules first (fast path)
        if user_source.parsing_rules.is_some() {
            info!("Using stored parsing rules for source: {}", source);
            match fetch_user_source_with_rules(&source, &user_source).await {
                Ok(news_source) => {
                    info!("Rules parsing successful for source: {}, got {} news items", source, news_source.items.len());
                    return Json(ApiResponse::success(news_source));
                }
                Err(e) => {
                    warn!("Rules parsing failed for source: {}, regenerating rules: {}", source, e);
                    
                    // Strategy 2: Regenerate rules and try again
                    let mut user_source_mut = user_source.clone();
                    match regenerate_parsing_rules(&source, &mut user_source_mut).await {
                        Ok(_) => {
                            // Save updated rules
                            let mut config_mut = config;
                            if let Some(index) = config_mut.user_sources.iter().position(|s| s.name == source) {
                                config_mut.user_sources[index] = user_source_mut.clone();
                                if let Err(save_err) = config_mut.save() {
                                    error!("Failed to save updated parsing rules: {}", save_err);
                                }
                            }
                            
                            // Try parsing with new rules
                            match fetch_user_source_with_rules(&source, &user_source_mut).await {
                                Ok(news_source) => {
                                    info!("New rules parsing successful for source: {}, got {} news items", source, news_source.items.len());
                                    return Json(ApiResponse::success(news_source));
                                }
                                Err(rules_err) => {
                                    warn!("New rules parsing also failed for source: {}, falling back to AI: {}", source, rules_err);
                                }
                            }
                        }
                        Err(regen_err) => {
                            warn!("Failed to regenerate rules for source: {}, falling back to AI: {}", source, regen_err);
                        }
                    }
                }
            }
        }
        
        // Strategy 3: Fallback to AI parsing
        info!("Using AI parsing fallback for source: {}", source);
        match fetch_user_source_with_ai(&source, &user_source.url).await {
            Ok(news_source) => {
                info!("AI parsing successful for source: {}, got {} news items", source, news_source.items.len());
                Json(ApiResponse::success(news_source))
            }
            Err(e) => {
                error!("All parsing strategies failed for source: {}", source);
                Json(ApiResponse::<NewsSource>::error(format!("User source parsing failed: {}", e)))
            }
        }
    } else {
        // Not a user source, use predefined sources logic
        let result = match source.as_str() {
            "bilibili" => state.news_service.get_bilibili_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "weibo" => state.news_service.get_weibo_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "zhihu" => state.news_service.get_zhihu_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "github" => state.news_service.get_github_trending(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "juejin" => state.news_service.get_juejin_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "douyin" => state.news_service.get_douyin_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "36kr" => state.news_service.get_36kr_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "ithome" => state.news_service.get_ithome_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "segmentfault" => state.news_service.get_segmentfault_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "oschina" => state.news_service.get_oschina_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "infoq" => state.news_service.get_infoq_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "ruanyifeng" => state.news_service.get_ruanyifeng_weekly(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "csdn" => state.news_service.get_csdn_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "stcn" => state.news_service.get_stcn_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "caixin" => state.news_service.get_caixin_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "baidu" => state.news_service.get_baidu_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            "toutiao" => state.news_service.get_toutiao_hot(no_cache).await.map_err(|e| anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Unsupported data source: {}", source)),
        };

        match result {
            Ok(news_source) => {
                info!("预定义数据源 {} ({})", source, news_source.items.len());
                Json(ApiResponse::success(news_source))
            }
            Err(e) => {
                warn!("预定义数据源获取失败 {} - {}", source, e);
                Json(ApiResponse::<NewsSource>::error(format!("数据源获取失败: {}", e)))
            }
        }
    }
}
// ...
// 
async fn get_combined_news(
    State(state): State<AppState>,
    Query(query): Query<CombinedNewsQuery>,
) -> impl IntoResponse {
    let no_cache = query.no_cache.unwrap_or(false);
    
    // 解析新闻源，默认使用百度+知乎+微博
    let sources_str = query.sources.unwrap_or_else(|| "baidu,zhihu,weibo".to_string());
    let sources: Vec<&str> = sources_str.split(',').map(|s| s.trim()).collect();
    
    // 限制最多3个新闻源
    let limited_sources: Vec<&str> = sources.into_iter().take(3).collect();
    
    info!("获取综合新闻: {:?}", limited_sources);
    
    let mut combined_results = serde_json::Map::new();
    let mut total_items = 0;
    
    for source in &limited_sources {
        let cache_key = format!("news:{}", source);
        
        // 检查缓存
        let result = if !no_cache {
            if let Some(cached) = state.cache.get(&cache_key) {
                if !cached.is_expired() {
                    info!("返回缓存的新闻数据: {}", source);
                    Some(Ok(cached.data.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        let data = if let Some(cached_result) = result {
            cached_result
        } else {
            // 获取新数据
            fetch_news_data(&state, source, no_cache).await
        };
        
        match data {
            Ok(news_data) => {
                if let Some(items) = news_data.get("items").and_then(|v| v.as_array()) {
                    total_items += items.len();
                }
                combined_results.insert(source.to_string(), news_data);
            }
            Err(e) => {
                warn!("获取{}新闻失败: {}", source, e);
                combined_results.insert(source.to_string(), serde_json::json!({
                    "error": format!("获取{}失败: {}", source, e),
                    "items": []
                }));
            }
        }
    }
    
    // 缓存综合结果
    if !no_cache {
        let combined_key = format!("combined:{}", sources_str);
        let combined_data = serde_json::Value::Object(combined_results.clone());
        let http_cache_ttl = state.config.get_http_cache_ttl();
        let cached = CachedResponse::new(combined_data.clone(), Duration::from_secs(http_cache_ttl));
        state.cache.insert(combined_key, cached);
    }
    
    info!("成功获取综合新闻 ({} 个源, {} 条)", limited_sources.len(), total_items);
    
    Json(ApiResponse::success(serde_json::json!({
        "sources": limited_sources,
        "total_items": total_items,
        "data": combined_results
    })))
}

// 辅助函数：获取单个新闻源的数据
async fn fetch_news_data(state: &AppState, source: &str, no_cache: bool) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let result = match source {
        "bilibili" => state.news_service.get_bilibili_hot(no_cache).await,
        "weibo" => state.news_service.get_weibo_hot(no_cache).await,
        "zhihu" => state.news_service.get_zhihu_hot(no_cache).await,
        "github" => state.news_service.get_github_trending(no_cache).await,
        "juejin" => state.news_service.get_juejin_hot(no_cache).await,
        "douyin" => state.news_service.get_douyin_hot(no_cache).await,
        "36kr" => state.news_service.get_36kr_hot(no_cache).await,
        "ithome" => state.news_service.get_ithome_hot(no_cache).await,
        "segmentfault" => state.news_service.get_segmentfault_hot(no_cache).await,
        "oschina" => state.news_service.get_oschina_hot(no_cache).await,
        "infoq" => state.news_service.get_infoq_hot(no_cache).await,
        "ruanyifeng" => state.news_service.get_ruanyifeng_weekly(no_cache).await,
        "csdn" => state.news_service.get_csdn_hot(no_cache).await,
        "stcn" => state.news_service.get_stcn_hot(no_cache).await,
        "caixin" => state.news_service.get_caixin_hot(no_cache).await,
        "baidu" => state.news_service.get_baidu_hot(no_cache).await,
        "toutiao" => state.news_service.get_toutiao_hot(no_cache).await,
        _ => return Err(format!("未知的新闻源: {}", source).into()),
    };
    
    match result {
        Ok(news_source) => {
            let data = serde_json::to_value(&news_source).unwrap_or_default();
            Ok(data)
        }
        Err(e) => Err(e)
    }
}

// 清除缓存
async fn clear_cache(State(state): State<AppState>) -> impl IntoResponse {
    // 清除 HTTP 响应缓存
    let http_cache_count = state.cache.len();
    state.cache.clear();
    
    // 清除新闻服务内部的缓存
    let news_cache_count = match state.news_service.clear_cache().await {
        Ok(count) => count,
        Err(e) => {
            warn!("清除新闻服务缓存失败: {}", e);
            0
        }
    };
    
    let total_count = http_cache_count + news_cache_count;
    info!("已清除 {} 个缓存项 (HTTP缓存: {}, 新闻缓存: {})", total_count, http_cache_count, news_cache_count);
    Json(ApiResponse::success(total_count))
}

// 图片代理
async fn proxy_image(
    Query(query): Query<ImageQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("图片代理请求: url='{}'", query.url);
    
    // 验证 URL 是否是有效的绝对 URL
    if !query.url.starts_with("http://") && !query.url.starts_with("https://") {
        warn!("图片 URL 不是有效的绝对路径: '{}'", query.url);
        return Err((StatusCode::BAD_REQUEST, format!("图片 URL 必须是有效的绝对路径 (http:// 或 https://)，当前: '{}'", query.url)));
    }
    
    // 从图片 URL 中提取域名作为 Referer
    let referer = if let Ok(url) = url::Url::parse(&query.url) {
        format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""))
    } else {
        query.url.clone()
    };
    
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("创建HTTP客户端失败: {}", e)))?;

    let request = client.get(&query.url)
        .header("Accept", "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", &referer)
        .header("sec-ch-ua", "\"Chromium\";v=\"123\", \"Not:A-Brand\";v=\"8\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
        .header("Sec-Fetch-Dest", "image")
        .header("Sec-Fetch-Mode", "no-cors")
        .header("Sec-Fetch-Site", "cross-site");

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                warn!("图片获取失败: {} - HTTP {}", query.url, status);
                return Err((StatusCode::BAD_GATEWAY, format!("图片获取失败: HTTP {}", status)));
            }

            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("image/jpeg")
                .to_string();

            match response.bytes().await {
                Ok(bytes) => {
                    info!("成功代理图片: {} ({} bytes)", query.url, bytes.len());
                    let mut resp = Response::new(axum::body::Body::from(bytes));
                    resp.headers_mut().insert(
                        header::CONTENT_TYPE,
                        header::HeaderValue::from_str(&content_type).unwrap_or_else(|_| header::HeaderValue::from_static("image/jpeg"))
                    );
                    // 添加 CORS 头
                    resp.headers_mut().insert(
                        header::ACCESS_CONTROL_ALLOW_ORIGIN,
                        header::HeaderValue::from_static("*")
                    );
                    Ok(resp)
                }
                Err(e) => {
                    warn!("读取图片数据失败: {} - {}", query.url, e);
                    return Err((StatusCode::BAD_GATEWAY, format!("读取图片失败: {}", e)));
                }
            }
        }
        Err(e) => {
            warn!("获取图片失败: {} - {}", query.url, e);
            return Err((StatusCode::BAD_GATEWAY, format!("获取图片失败: {}", e)));
        }
    }
}

// 首页
async fn index() -> impl IntoResponse {
    let html = r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>NewsLatest API</title>
    <link rel="icon" type="image/x-icon" href="/icon.ico">
    <link rel="shortcut icon" href="/icon.ico">
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; line-height: 1.6; }
        .header { text-align: center; margin-bottom: 40px; }
        .endpoint { background: #f8f9fa; padding: 20px; margin: 20px 0; border-radius: 8px; border-left: 4px solid #007bff; }
        .method { color: #007bff; font-weight: bold; font-family: monospace; }
        .url { color: #28a745; font-family: monospace; background: #f1f3f4; padding: 2px 6px; border-radius: 4px; }
        .description { color: #6c757d; margin: 8px 0; }
        pre { background: #f8f9fa; padding: 12px; border-radius: 4px; overflow-x: auto; }
        .news-sources { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px; margin: 20px 0; }
        .source { background: #e9ecef; padding: 10px; border-radius: 4px; text-align: center; cursor: pointer; transition: all 0.3s ease; }
        .source:hover { background: #007bff; color: white; transform: translateY(-2px); box-shadow: 0 4px 8px rgba(0,123,255,0.3); }
        .source a { text-decoration: none; color: inherit; display: block; }
    </style>
</head>
<body>
    <div class="header">
        <h1>📰 NewsLatest API</h1>
        <p>基于 Rust + Axum 的新闻聚合 API 服务</p>
    </div>
    
    <div class="endpoint">
        <div><span class="method">GET</span> <span class="url">/news/{source}</span></div>
        <div class="description">获取指定新闻源的数据<span style="color: #dc3545;">  (GET /news/bilibili?no_cache=false)</span></div>
    </div>
    
    <div class="endpoint">
        <div><span class="method">GET</span> <span class="url">/news/combined</span></div>
        <div class="description">获取综合新闻数据<span style="color: #dc3545;">  (GET /news/combined?sources=baidu,zhihu,weibo&no_cache=false)</span></div>
        <div class="description">支持自定义新闻源（最多3个），默认：百度+知乎+微博</div>
    </div>
    
    <h2>📰 支持的新闻源</h2>
    <div class="news-sources">
        <div class="source" style="background: #007bff; color: white;"><a href="/news/combined?no_cache=false" target="_blank" style="color: white;">🔥 综合新闻</a></div>
        <div class="source"><a href="/news/bilibili?no_cache=false" target="_blank">bilibili</a></div>
        <div class="source"><a href="/news/weibo?no_cache=false" target="_blank">weibo</a></div>
        <div class="source"><a href="/news/zhihu?no_cache=false" target="_blank">zhihu</a></div>
        <div class="source"><a href="/news/github?no_cache=false" target="_blank">github</a></div>
        <div class="source"><a href="/news/juejin?no_cache=false" target="_blank">juejin</a></div>
        <div class="source"><a href="/news/douyin?no_cache=false" target="_blank">douyin</a></div>
        <div class="source"><a href="/news/36kr?no_cache=false" target="_blank">36kr</a></div>
        <div class="source"><a href="/news/ithome?no_cache=false" target="_blank">ithome</a></div>
        <div class="source"><a href="/news/segmentfault?no_cache=false" target="_blank">segmentfault</a></div>
        <div class="source"><a href="/news/oschina?no_cache=false" target="_blank">oschina</a></div>
        <div class="source"><a href="/news/infoq?no_cache=false" target="_blank">infoq</a></div>
        <div class="source"><a href="/news/ruanyifeng?no_cache=false" target="_blank">ruanyifeng</a></div>
        <div class="source"><a href="/news/csdn?no_cache=false" target="_blank">csdn</a></div>
        <div class="source"><a href="/news/stcn?no_cache=false" target="_blank">stcn</a></div>
        <div class="source"><a href="/news/caixin?no_cache=false" target="_blank">caixin</a></div>
        <div class="source"><a href="/news/baidu?no_cache=false" target="_blank">baidu</a></div>
        <div class="source"><a href="/news/toutiao?no_cache=false" target="_blank">toutiao</a></div>
    </div>
</body>
</html>
    "#;
    
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}

// 图标服务
async fn serve_icon() -> impl IntoResponse {
    // 尝试读取图标文件
    match tokio::fs::read("icon.ico").await {
        Ok(icon_data) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, "image/x-icon")], icon_data)
        }
        Err(_) => {
            (StatusCode::NOT_FOUND, [(header::CONTENT_TYPE, "text/plain")], "Icon not found".to_string().into_bytes())
        }
    }
}

// // 获取用户数据源列表
// async fn get_user_sources(State(state): State<AppState>) -> impl IntoResponse {
//     let user_manager = state.user_source_manager.lock().unwrap();
//     let sources = user_manager.get_sources();
//     Json(ApiResponse::success(sources))
// }

// 创建用户数据源
async fn create_user_source(
    State(state): State<AppState>,
    Json(request): Json<CreateUserSourceRequest>,
) -> impl IntoResponse {
    info!("========== 开始创建用户数据源 ==========");
    info!("请求参数: name={}, source_type={}, url={}", request.name, request.source_type, request.url);
    
    // 1. 创建数据源
    let source = {
        let mut user_manager = state.user_source_manager.lock().unwrap();
        match user_manager.add_source(request) {
            Ok(source) => source,
            Err(e) => {
                error!("创建数据源失败: {}", e);
                return Json(ApiResponse::error(e.to_string()));
            }
        }
    };
    
    info!("数据源创建成功: id={}, name={}", source.id, source.name);
    
    // 2. 根据 source_type 调用 AI 生成规则
    let mut source_with_rules = source.clone();
    
    match source.source_type {
        types::UserSourceType::Web => {
            info!("检测到 HTML 模式数据源，准备调用 AI 生成解析规则...");
            
            // 初始化 AI 客户端
            let ai_client = match AIClient::new().await {
                Ok(client) => {
                    info!("AI 客户端初始化成功");
                    client
                },
                Err(e) => {
                    warn!("AI 客户端初始化失败: {}，跳过规则生成", e);
                    return Json(ApiResponse::success(source));
                }
            };
            
            // 检查 AI 是否启用
            if !ai_client.is_enabled() {
                warn!("AI 功能未启用，跳过规则生成");
                return Json(ApiResponse::success(source));
            }
            
            info!("AI 功能已启用，provider: {}", ai_client.get_provider_name());
            
            // 获取 HTML 内容
            let scraper = WebScraper::new();
            let html_content = if let Some(selector_snippet) = &source.selector {
                info!("使用提供的 selector 片段 ({} 字节)", selector_snippet.len());
                selector_snippet.clone()
            } else {
                info!("从 URL 获取 HTML 内容: {}", source.url);
                match scraper.fetch_html(&source.url, true).await {
                    Ok(content) => {
                        info!("成功获取 HTML 内容 ({} 字节)", content.len());
                        content
                    },
                    Err(e) => {
                        warn!("获取 HTML 内容失败: {}，跳过规则生成", e);
                        return Json(ApiResponse::success(source));
                    }
                }
            };
            
            // 调用 AI 生成结构化提取规则
            info!("开始调用 AI 生成结构化提取规则...");
            match ai_client.generate_structured_extraction_rules(&source.url, &html_content).await {
                Ok(rules) => {
                    info!("AI 成功生成结构化提取规则: item_node={}", rules.selectors.item_node);
                    info!("生成的字段: {:?}", rules.selectors.fields.keys().collect::<Vec<_>>());
                    source_with_rules.parsing_rules = Some(types::ParsingRulesVariant::Structured(rules));
                },
                Err(e) => {
                    warn!("AI 生成结构化提取规则失败: {}", e);
                }
            }
        }
        types::UserSourceType::Json => {
            info!("检测到 API 模式数据源，准备调用 AI 生成字段映射规则...");
            
            // 初始化 AI 客户端
            let ai_client = match AIClient::new().await {
                Ok(client) => {
                    info!("AI 客户端初始化成功");
                    client
                },
                Err(e) => {
                    warn!("AI 客户端初始化失败: {}，跳过规则生成", e);
                    return Json(ApiResponse::success(source));
                }
            };
            
            // 检查 AI 是否启用
            if !ai_client.is_enabled() {
                warn!("AI 功能未启用，跳过规则生成");
                return Json(ApiResponse::success(source));
            }
            
            info!("AI 功能已启用，provider: {}", ai_client.get_provider_name());
            
            // 获取 JSON 内容
            let scraper = WebScraper::new();
            info!("从 URL 获取 JSON 内容: {}", source.url);
            match scraper.fetch_html_direct(&source.url, false).await {
                Ok(json_content) => {
                    info!("成功获取 JSON 内容 ({} 字节)", json_content.len());
                    
                    // 调用 AI 生成字段映射规则
                    info!("开始调用 AI 生成字段映射规则...");
                    match ai_client.generate_field_mapping_rules(&json_content, &source.url).await {
                        Ok(rules) => {
                            info!("AI 成功生成字段映射规则");
                            source_with_rules.field_mapping_rules = Some(rules);
                        },
                        Err(e) => {
                            warn!("AI 生成字段映射规则失败: {}", e);
                        }
                    }
                },
                Err(e) => {
                    warn!("获取 JSON 内容失败: {}，跳过规则生成", e);
                }
            }
        }
    }
    
    // 3. 保存更新后的数据源（包含规则）
    if source_with_rules.parsing_rules.is_some() || source_with_rules.field_mapping_rules.is_some() {
        info!("保存生成的规则到数据源配置...");
        let mut user_manager = state.user_source_manager.lock().unwrap();
        if let Err(e) = user_manager.update_source(source_with_rules.clone()) {
            warn!("保存规则失败: {}", e);
        } else {
            info!("规则保存成功");
        }
    }
    
    info!("========== 用户数据源创建完成 ==========");
    Json(ApiResponse::success(source_with_rules))
}

// // 删除用户数据源
async fn delete_user_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut user_manager = state.user_source_manager.lock().unwrap();
    match user_manager.remove_source(&id) {
        Ok(Some(source)) => Json(ApiResponse::success(source)),
        Ok(None) => Json(ApiResponse::error("数据源不存在".to_string())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber first, before anything else, so that
    // startup errors and panics are captured in the deployment logs.
    // Log level is controlled by the RUST_LOG environment variable (default: info).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    // Install a panic hook that logs the panic via tracing before aborting,
    // so the message is visible in Railway's log stream.
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic payload".to_string()
        };
        tracing::error!("PANIC at {}: {}", location, message);
        default_panic(info);
    }));

    info!("🚀 Tracing initialized — newslatest-server is starting up");
    info!("📰 启动 NewsLatest 服务器...");


    // 确保配置目录存在
    if let Err(e) = std::fs::create_dir_all("config") {
        warn!("⚠️ 创建 config 目录失败: {}，将使用默认配置", e);
    }

    // 确保数据目录存在
    if let Err(e) = std::fs::create_dir_all("data") {
        warn!("⚠️ 创建 data 目录失败: {}，用户数据源可能无法保存", e);
    }

    // 创建应用状态
    let config = match config::Config::load() {
        Ok(config) => {
            info!("✅ 配置文件加载成功");
            Arc::new(config)
        }
        Err(e) => {
            warn!("⚠️ 配置文件加载失败: {}，使用默认配置", e);
            Arc::new(config::Config::default())
        }
    };

    // 获取端口配置
    let config_port = config.get_port();

    let news_service = Arc::new(NewsService::with_config((*config).clone()));

    // 初始化用户数据源管理器
    let user_source_manager = match UserSourceManager::new("data/user_sources.json") {
        Ok(manager) => Arc::new(Mutex::new(manager)),
        Err(e) => {
            error!("❌ 初始化用户数据源管理器失败: {}", e);
            return Err(e);
        }
    };

    let app_state = AppState {
        news_service,
        cache: Arc::new(DashMap::new()),
        config,
        user_source_manager,
    };

    // 创建路由
    let app = Router::new()
        // API路由
        .route("/api/health", get(health_check))
        .route("/api/news/:source", get(get_news_simple))
        .route("/api/news/combined", get(get_combined_news))
        .route("/api/cache", delete(clear_cache))
        .route("/api/proxy/image", get(proxy_image))
        // AI配置管理API
        .route("/api/ai-config", get(get_ai_config).post(update_ai_config))
        .route("/api/ai-providers", get(get_ai_providers))
        .route("/api/ai-test", post(test_ai_connection))
        // 用户数据源管理API
        .route("/api/user-sources", get(get_user_sources).post(create_user_source))
        .route("/api/user-sources/:id", delete(delete_user_source))
        .route("/api/user-sources/generate-rules", post(generate_parsing_rules))
        .route("/api/user-sources/generate-structured-rules", post(generate_structured_extraction_rules))
        // 静态文件和首页
        .route("/", get(index))
        .route("/icon.ico", get(serve_icon))
        .fallback(index)
        // CORS配置
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
                .allow_credentials(false),
        )
                .with_state(app_state);

    // 启动服务器
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| config_port.to_string())
        .parse()
        .unwrap_or_else(|_| config_port);
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.expect("绑定地址失败");
    info!("🌐 服务器启动在 http://0.0.0.0:{}", port);
    info!("📋 API文档: http://IP:{}", port);
    info!("🚀 前端应用请运行: npm run dev");

    axum::serve(listener, app).await?;  

    Ok(())
}
