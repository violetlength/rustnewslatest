use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub desc: Option<String>,
    pub cover: Option<String>,
    pub author: Option<String>,
    pub timestamp: Option<String>,
    pub hot: Option<u64>,
    pub url: String,
    pub mobile_url: Option<String>,
}

impl Default for NewsItem {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            desc: None,
            cover: None,
            author: None,
            timestamp: None,
            hot: None,
            url: String::new(),
            mobile_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsSource {
    pub name: String,
    pub title: String,
    pub description: String,
    pub link: String,
    pub items: Vec<NewsItem>,
    pub total: usize,
    pub from_cache: bool,
    pub update_time: String,
    pub expires_at: Option<String>,
    pub ttl_minutes: Option<u64>,
}

impl NewsSource {
    pub fn new(name: String, title: String, description: String, link: String) -> Self {
        Self {
            name,
            title,
            description,
            link,
            items: Vec::new(),
            total: 0,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: None,
            ttl_minutes: None,
        }
    }

    pub fn add_item(&mut self, item: NewsItem) {
        self.items.push(item);
        self.total = self.items.len();
    }

    pub fn set_cache_info(&mut self, expires_at: DateTime<Utc>, ttl_minutes: u64) {
        self.expires_at = Some(expires_at.to_rfc3339());
        self.ttl_minutes = Some(ttl_minutes);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserSourceType {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "web")]
    Web,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsingRules {
    pub container_selector: String,    // Container selector for news items (e.g., "#content li", "article")
    pub title_in_container: String,    // Title selector within container (e.g., "a", "h2")
    pub link_in_container: String,      // Link selector within container (e.g., "a", "h2 a", ".link") 
    pub desc_in_container: Option<String>, // Description selector within container (e.g., "p", ".desc", ".summary")
    pub time_in_container: Option<String>, // Time selector within container (e.g., "span", ".time", "time")
    pub created_at: DateTime<Utc>,      // Rules generation time
    pub success_rate: f64,             // Success rate (0.0-1.0)
    pub total_attempts: u64,           // Total parsing attempts
}

/// Field extraction rule for structured data extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldRule {
    pub selector: String,              // CSS selector for the field
    pub attribute: String,             // Attribute to extract ("text", "href", "src", etc.)
    pub required: bool,                // Whether this field is required
    pub base_url: Option<String>,      // Base URL for relative links (optional)
    pub format: Option<String>,        // Format type ("datetime", "url", etc.)
    pub clean: bool,                   // Whether to clean whitespace
}

/// Structured extraction rules for precise data extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRules {
    pub task: String,                  // Task description (e.g., "extract_news_list")
    pub source_url: String,            // Source URL these rules apply to
    pub rules_version: String,         // Version of the rules
    pub selectors: ExtractionSelectors, // Main selectors structure
    pub notes: Option<String>,         // Additional notes
    pub created_at: DateTime<Utc>,     // Rules creation time
    pub success_rate: f64,             // Success rate (0.0-1.0)
    pub total_attempts: u64,           // Total parsing attempts
}

/// Main selectors structure for extraction rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSelectors {
    pub root_container: String,        // Root container selector (e.g., "div#newslist")
    pub item_list: String,             // List container selector (e.g., "ul#content")
    pub item_node: String,             // Individual item selector (e.g., "li")
    pub fields: std::collections::HashMap<String, FieldRule>, // Field extraction rules
}

/// Legacy parsing rules compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParsingRulesVariant {
    Legacy(ParsingRules),
    Structured(ExtractionRules),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNewsSource {
    pub id: String,
    pub name: String,
    pub title: String,
    pub description: String,
    pub source_type: UserSourceType,
    pub url: String,
    pub selector: Option<String>, // CSS selector for web sources (backward compatibility)
    pub parsing_rules: Option<ParsingRulesVariant>, // AI-generated parsing rules (legacy or structured)
    pub created_at: DateTime<Utc>,
    pub user_id: Option<String>,
    pub is_active: bool,
}

impl UserNewsSource {
    pub fn new(
        name: String,
        title: String,
        description: String,
        source_type: UserSourceType,
        url: String,
        selector: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            title,
            description,
            source_type,
            url,
            selector,
            parsing_rules: None, // Will be generated later
            created_at: Utc::now(),
            user_id: None,
            is_active: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSourcesConfig {
    pub user_sources: Vec<UserNewsSource>,
}

impl UserSourcesConfig {
    pub fn new() -> Self {
        Self {
            user_sources: Vec::new(),
        }
    }

    pub async fn load() -> Result<Self, std::io::Error> {
        let config_path = "data/user_sources.json";
        let content = std::fs::read_to_string(config_path)?;
        let config: UserSourcesConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_path = "data/user_sources.json";
        let content = serde_json::to_string_pretty(&self)?;
        
        // Ensure directory exists
        if let Some(parent) = std::path::Path::new(config_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(config_path, content)?;
        Ok(())
    }

    pub fn add_source(&mut self, source: UserNewsSource) {
        self.user_sources.push(source);
    }

    pub fn remove_source(&mut self, id: &str) -> Option<UserNewsSource> {
        let index = self.user_sources.iter().position(|s| s.id == id)?;
        Some(self.user_sources.remove(index))
    }

    pub fn find_source(&self, id: &str) -> Option<&UserNewsSource> {
        self.user_sources.iter().find(|s| s.id == id)
    }

    pub fn find_source_by_name(&self, name: &str) -> Option<&UserNewsSource> {
        self.user_sources.iter().find(|s| s.name == name)
    }

    pub fn get_active_sources(&self) -> Vec<&UserNewsSource> {
        self.user_sources.iter().filter(|s| s.is_active).collect()
    }
}

// 用于创建用户数据源的请求结构
#[derive(Debug, Deserialize)]
pub struct CreateUserSourceRequest {
    pub name: String,
    pub title: String,
    pub description: String,
    pub source_type: String, // "json" 或 "web"
    pub url: String,
    pub selector: Option<String>,
}
