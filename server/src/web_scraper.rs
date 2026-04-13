use anyhow::{Result, anyhow};
use reqwest::Client;
use std::time::Duration;
use tracing::{info, warn, debug};
use chrono::TimeZone;
use scraper::{Html, Selector};
use std::collections::HashMap;

pub struct WebScraper {
    client: Client,
}

impl WebScraper {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub async fn fetch_html(&self, url: &str, use_headless: bool) -> Result<String> {
        info!("Fetching HTML from: {}", url);

        // Try RSS feeds first  咱不获取rss
        // let rss_urls = [
        //     format!("{}/rss", url),
        //     format!("{}/rss.xml", url),
        //     format!("{}/feed", url),
        //     format!("{}/feed.xml", url),
        // ];

        // for rss_url in &rss_urls {
        //     if let Ok(content) = self.try_fetch_rss(rss_url).await {
        //         info!("Found RSS feed at: {}", rss_url);
        //         return Ok(content);
        //     }
        // }

        // Fall back to HTML
        self.fetch_html_direct(url, use_headless).await
    }

    async fn try_fetch_rss(&self, url: &str) -> Result<String> {
        let response = self.client
            .get(url)
            .header("Accept", "application/rss+xml, application/xml, text/xml")
            .send()
            .await?;

        let content = response.text().await?;
        
        if content.contains("<rss") || content.contains("<feed") {
            Ok(content)
        } else {
            Err(anyhow!("Not an RSS feed"))
        }
    }

    pub async fn fetch_html_direct(&self, url: &str, use_headless: bool) -> Result<String> {
        if use_headless {
            info!("Fetching HTML with headless browser: {}", url);
            self.fetch_html_with_headless_browser(url).await
        } else {
            // Check if this is a JSON API
            let accept_header = if url.contains("/api/") || url.contains("?") {
                info!("Fetching JSON API directly with HTTP: {}", url);
                "application/json, text/plain, */*"
            } else {
                info!("Fetching HTML directly with HTTP: {}", url);
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"
            };
            
            let response = self.client
                .get(url)
                .header("Accept", accept_header)
                .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
                .send()
                .await
                .map_err(|e| anyhow!("Failed to fetch content: {}", e))?;

            let content = response.text().await
                .map_err(|e| anyhow!("Failed to read response text: {}", e))?;

            info!("Fetched content, length: {}", content.len());
            Ok(content)
        }
    }

    /// Fetch HTML with headless browser
    async fn fetch_html_with_headless_browser(&self, url: &str) -> Result<String> {
        use headless_chrome::{Browser, LaunchOptions};
        use std::time::Duration;

        info!("Launching headless browser...");

        let browser = Browser::new(
            LaunchOptions::default_builder()
                .headless(true)
                .build()
                .map_err(|e| anyhow!("Failed to launch browser: {}", e))?
        ).map_err(|e| anyhow!("Failed to create browser: {}", e))?;

        let tab = browser.new_tab()
            .map_err(|e| anyhow!("Failed to create tab: {}", e))?;

        info!("Navigating to: {}", url);
        tab.navigate_to(url)
            .map_err(|e| anyhow!("Failed to navigate: {}", e))?;

        // Wait for page to load and render
        info!("Waiting for page to render (5 seconds)...");
        std::thread::sleep(Duration::from_secs(5));

        let html = tab.get_content()
            .map_err(|e| anyhow!("Failed to get content: {}", e))?;

        // Browser will be closed automatically when dropped
        drop(browser);

        info!("Fetched HTML using headless browser, length: {}", html.len());
        
        // Print preview for debugging
        let preview: String = html.chars().take(500).collect();
        info!("HTML preview (first 500 chars): {}", preview);
        
        let has_article = html.contains("<article") || html.contains("article");
        let has_script = html.contains("<script");
        let has_content = html.len() > 10000;
        info!("HTML analysis - has_article: {}, has_script: {}, has_content: {}", 
              has_article, has_script, has_content);
        
        Ok(html)
    }

    pub async fn parse_news_with_rules(
        &self,
        html_content: &str,
        rules: &crate::types::ParsingRules,
    ) -> Result<Vec<crate::types::NewsItem>> {
        use scraper::{Html, Selector};
        
        let document = Html::parse_document(html_content);
        
        // Parse container selector for news items
        let container_selector = Selector::parse(&rules.container_selector)
            .map_err(|e| anyhow!("Invalid container selector '{}': {}", rules.container_selector, e))?;
        
        // Parse selectors within containers
        let title_selector = Selector::parse(&rules.title_in_container)
            .map_err(|e| anyhow!("Invalid title selector '{}': {}", rules.title_in_container, e))?;
        
        let link_selector = Selector::parse(&rules.link_in_container)
            .map_err(|e| anyhow!("Invalid link selector '{}': {}", rules.link_in_container, e))?;
        
        let desc_selector = rules.desc_in_container.as_ref()
            .map(|s| Selector::parse(s))
            .transpose()
            .map_err(|e| anyhow!("Invalid desc selector: {}", e))?;
        
        let time_selector = rules.time_in_container.as_ref()
            .map(|s| Selector::parse(s))
            .transpose()
            .map_err(|e| anyhow!("Invalid time selector: {}", e))?;
        
        let mut news_items = Vec::new();
        
        // For each container element, extract data using the intra-container selectors
        for container in document.select(&container_selector) {
            // Extract title from within container
            let title = container.select(&title_selector)
                .next()
                .and_then(|el| el.text().next())
                .unwrap_or_default()
                .trim()
                .to_string();
            
            // Extract link from within container
            let link = container.select(&link_selector)
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or_default()
                .to_string();
            
            // Extract description from within container (optional)
            let description = desc_selector.as_ref()
                .and_then(|selector| {
                    container.select(selector).next()
                        .and_then(|el| el.text().next())
                })
                .unwrap_or_default()
                .trim()
                .to_string();
            
            // Extract time from within container (optional)
            let time_str = time_selector.as_ref()
                .and_then(|selector| {
                    container.select(selector).next()
                        .and_then(|el| el.text().next())
                });
            
            let timestamp = if let Some(time_str) = time_str {
                self.parse_time(&time_str).unwrap_or_else(|_| {
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
                })
            } else {
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
            };
            
            // Skip items without title or link
            if title.is_empty() || link.is_empty() {
                continue;
            }
            
            let news_item = crate::types::NewsItem {
                id: uuid::Uuid::new_v4().to_string(),
                title,
                desc: Some(description),
                cover: None,
                author: None,
                timestamp: Some(timestamp),
                hot: None,
                url: link,
                mobile_url: None,
            };
            
            news_items.push(news_item);
        }
        
        info!("Parsed {} news items using rules", news_items.len());
        Ok(news_items)
    }
    
    fn parse_time(&self, time_str: &str) -> Result<String> {
        use chrono::{DateTime, Local, NaiveDateTime, Utc};
        use regex::Regex;
        
        let time_str = time_str.trim();
        
        // Try various time formats
        let formats = [
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d %H:%M",
            "%Y-%m-%d",
            "%m-%d %H:%M",
            "%H:%M",
            "%Y/%m/%d %H:%M:%S",
            "%Y/%m/%d %H:%M",
            "%Y/%m/%d",
        ];
        
        for format in &formats {
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(time_str, format) {
                let local_dt = Local.from_local_datetime(&naive_dt).single()
                    .unwrap_or_else(|| Local::now());
                return Ok(local_dt.with_timezone(&Utc).format("%Y-%m-%d %H:%M:%S").to_string());
            }
        }
        
        // Try relative time patterns
        if let Ok(re) = Regex::new(r"(\d+)(?:\s*)(?:minute|minutes|min|hour|hours|hr|day|days|d)\s*ago") {
            if let Some(caps) = re.captures(time_str) {
                if let Some(num_str) = caps.get(1) {
                    if let Ok(num) = num_str.as_str().parse::<i64>() {
                        let now = Local::now();
                        let past_time = if time_str.contains("minute") || time_str.contains("min") {
                            now - chrono::Duration::minutes(num)
                        } else if time_str.contains("hour") || time_str.contains("hr") {
                            now - chrono::Duration::hours(num)
                        } else if time_str.contains("day") || time_str.contains("d") {
                            now - chrono::Duration::days(num)
                        } else {
                            now
                        };
                        return Ok(past_time.with_timezone(&Utc).format("%Y-%m-%d %H:%M:%S").to_string());
                    }
                }
            }
        }
        
        // Default to current time
        Ok(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
    }

    /// Parse hot value from string like "100 points", "10k views", "1.2k" into u64
    fn parse_hot_value(&self, value: &str) -> Option<u64> {
        let value = value.trim().to_lowercase();
        
        // Extract numeric part with optional k/m suffix
        let re = regex::Regex::new(r"([\d.]+)\s*([km]?)").ok()?;
        
        if let Some(caps) = re.captures(&value) {
            let num_str = caps.get(1)?.as_str();
            let suffix = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            
            if let Ok(num) = num_str.parse::<f64>() {
                let multiplier = match suffix {
                    "k" => 1000.0,
                    "m" => 1_000_000.0,
                    _ => 1.0,
                };
                
                return Some((num * multiplier) as u64);
            }
        }
        
        // Fallback: try to parse any integer from the string
        let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.parse::<u64>().ok()
    }

    /// Parse news using structured extraction rules
    pub async fn parse_news_with_structured_rules(
        &self,
        html_content: &str,
        rules: &crate::types::ExtractionRules,
    ) -> Result<Vec<crate::types::NewsItem>> {
        let document = Html::parse_document(html_content);
        
        // Parse item nodes directly from entire document
        // Skip root_container and item_list to be more flexible
        let item_node_selector = Selector::parse(&rules.selectors.item_node)
            .map_err(|e| anyhow!("Invalid item node selector '{}': {}", rules.selectors.item_node, e))?;
        
        info!("Parsing with item_node selector: {}", rules.selectors.item_node);
        
        let mut news_items = Vec::new();
        
        // Try to find items in root_container first (if exists), otherwise search entire document
        let search_context: Box<dyn Iterator<Item = scraper::ElementRef>> = 
            if !rules.selectors.root_container.is_empty() && rules.selectors.root_container != "body" {
                // Try to find root container
                if let Ok(root_selector) = Selector::parse(&rules.selectors.root_container) {
                    if let Some(root) = document.select(&root_selector).next() {
                        info!("Found root_container: {}", rules.selectors.root_container);
                        Box::new(root.select(&item_node_selector))
                    } else {
                        // Root container not found, search entire document
                        info!("Root container '{}' not found, searching entire document", rules.selectors.root_container);
                        Box::new(document.select(&item_node_selector))
                    }
                } else {
                    Box::new(document.select(&item_node_selector))
                }
            } else {
                // No root_container specified, search entire document
                Box::new(document.select(&item_node_selector))
            };
        
        for item_node in search_context {
            let mut news_item = crate::types::NewsItem::default();
            let mut has_required_fields = true;
            
            // Extract each field according to rules
            for (field_name, field_rule) in &rules.selectors.fields {
                let field_selector = Selector::parse(&field_rule.selector)
                    .map_err(|e| anyhow!("Invalid field selector '{}': {}", field_rule.selector, e))?;
                
                if let Some(element) = item_node.select(&field_selector).next() {
                    let value = if field_rule.attribute == "text" {
                        let text = element.text().collect::<String>();
                        if field_rule.clean {
                            text.trim().to_string()
                        } else {
                            text
                        }
                    } else {
                        element.value().attr(&field_rule.attribute)
                            .unwrap_or_default()
                            .to_string()
                    };
                    
                    // Handle URL formatting for links
                    if field_name == "url" || field_name == "link" {
                        let base_url = field_rule.base_url.as_deref().unwrap_or(&rules.source_url);
                        let final_value = self.resolve_url(&value, base_url);
                        news_item.url = final_value;
                    } else if field_name == "title" {
                        news_item.title = value;
                    } else if field_name == "description" || field_name == "desc" {
                        news_item.desc = Some(value);
                    } else if field_name == "time" || field_name == "publish_time" || field_name == "timestamp" {
                        let formatted_time = if let Some(format_type) = &field_rule.format {
                            if format_type == "datetime" {
                                self.parse_time(&value).unwrap_or_else(|_| {
                                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
                                })
                            } else {
                                value
                            }
                        } else {
                            self.parse_time(&value).unwrap_or_else(|_| {
                                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
                            })
                        };
                        news_item.timestamp = Some(formatted_time);
                    } else if field_name == "cover" || field_name == "image" {
                        // Handle image URLs - resolve relative URLs
                        let base_url = field_rule.base_url.as_deref().unwrap_or(&rules.source_url);
                        let final_url = self.resolve_url(&value, base_url);
                        news_item.cover = Some(final_url);
                    } else if field_name == "author" {
                        news_item.author = Some(value);
                    } else if field_name == "hot" || field_name == "score" || field_name == "popularity" {
                        // Parse hot value from string like "100 points", "10k views", "1.2k"
                        let hot_value = self.parse_hot_value(&value);
                        news_item.hot = hot_value;
                    } else if field_name == "mobile_url" {
                        // Handle mobile URLs - resolve relative URLs
                        let base_url = field_rule.base_url.as_deref().unwrap_or(&rules.source_url);
                        let final_url = self.resolve_url(&value, base_url);
                        news_item.mobile_url = Some(final_url);
                    }
                } else if field_rule.required {
                    has_required_fields = false;
                    warn!("Required field '{}' not found in item", field_name);
                    break;
                }
            }
            
            // Only add item if it has required fields and a title
            if has_required_fields && !news_item.title.is_empty() {
                // Generate ID if not present
                if news_item.id.is_empty() {
                    news_item.id = format!("{}_{}", 
                        rules.source_url.replace("https://", "").replace("http://", "").replace('/', "_"),
                        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
                    );
                }
                news_items.push(news_item);
            }
        }
        
        info!("Parsed {} news items using structured rules", news_items.len());
        Ok(news_items)
    }

    fn resolve_url(&self, url: &str, base_url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else if url.starts_with("//") {
            if base_url.starts_with("https://") {
                format!("https:{}", url)
            } else {
                format!("http:{}", url)
            }
        } else if url.starts_with('/') {
            if let Ok(base) = url::Url::parse(base_url) {
                format!("{}://{}{}", base.scheme(), base.host_str().unwrap_or(""), url)
            } else {
                format!("{}{}", base_url.trim_end_matches('/'), url)
            }
        } else if url.is_empty() {
            String::new()
        } else {
            if let Some(last_slash) = base_url.rfind('/') {
                format!("{}{}", &base_url[..last_slash + 1], url)
            } else {
                format!("{}/{}", base_url, url)
            }
        }
    }
}
