use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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
