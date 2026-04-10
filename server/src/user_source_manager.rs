use crate::types::{UserNewsSource, UserSourcesConfig, UserSourceType, NewsSource, NewsItem, CreateUserSourceRequest};
use anyhow::{Result, anyhow};
use chrono::Utc;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tracing::{info, error, warn};

pub struct UserSourceManager {
    storage_path: String,
    storage: UserSourcesConfig,
}

impl UserSourceManager {
    pub fn new<P: AsRef<Path>>(storage_path: P) -> Result<Self> {
        let storage_path = storage_path.as_ref().to_string_lossy().to_string();
        
        // 确保目录存在
        if let Some(parent) = Path::new(&storage_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let storage = if Path::new(&storage_path).exists() {
            let content = fs::read_to_string(&storage_path)?;
            serde_json::from_str::<UserSourcesConfig>(&content)?
        } else {
            UserSourcesConfig::new()
        };

        Ok(Self {
            storage_path,
            storage,
        })
    }

    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.storage)?;
        fs::write(&self.storage_path, content)?;
        info!("用户数据源配置已保存到: {}", self.storage_path);
        Ok(())
    }

    pub fn add_source(&mut self, request: CreateUserSourceRequest) -> Result<UserNewsSource> {
        // 验证名称唯一性
        if self.storage.find_source_by_name(&request.name).is_some() {
            return Err(anyhow!("数据源名称 '{}' 已存在", request.name));
        }

        // 验证URL格式
        if !request.url.starts_with("http://") && !request.url.starts_with("https://") {
            return Err(anyhow!("URL 必须以 http:// 或 https:// 开头"));
        }

        let source_type = match request.source_type.as_str() {
            "json" => UserSourceType::Json,
            "web" => {
                if request.selector.is_none() {
                    return Err(anyhow!("网页类型数据源必须提供选择器"));
                }
                UserSourceType::Web
            },
            _ => return Err(anyhow!("不支持的数据源类型: {}", request.source_type)),
        };

        let source = UserNewsSource::new(
            request.name,
            request.title,
            request.description,
            source_type,
            request.url,
            request.selector,
        );

        self.storage.add_source(source.clone());
        self.save()?;
        
        info!("添加用户数据源: {} ({})", source.name, source.title);
        Ok(source)
    }

    pub fn remove_source(&mut self, id: &str) -> Result<Option<UserNewsSource>> {
        let source = self.storage.remove_source(id);
        if source.is_some() {
            self.save()?;
            info!("删除用户数据源: {}", id);
        }
        Ok(source)
    }

    pub fn get_sources(&self) -> Vec<UserNewsSource> {
        self.storage.user_sources.clone()
    }

    pub fn get_active_sources(&self) -> Vec<UserNewsSource> {
        self.storage.get_active_sources().into_iter().cloned().collect()
    }

    pub fn find_source(&self, name: &str) -> Option<UserNewsSource> {
        self.storage.find_source_by_name(name).cloned()
    }

    // 获取用户数据源的新闻内容
    pub async fn fetch_user_source_news(&self, source: &UserNewsSource, no_cache: bool) -> Result<NewsSource> {
        match &source.source_type {
            UserSourceType::Json => self.fetch_json_source(source).await,
            UserSourceType::Web => self.fetch_web_source(source).await,
        }
    }

    async fn fetch_json_source(&self, source: &UserNewsSource) -> Result<NewsSource> {
        let client = reqwest::Client::new();
        let response = client.get(&source.url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP请求失败: {}", response.status()));
        }

        let json_data: Value = response.json().await?;
        
        let mut news_source = NewsSource::new(
            source.name.clone(),
            source.title.clone(),
            source.description.clone(),
            source.url.clone(),
        );

        // 尝试解析常见的JSON格式
        if let Some(items) = self.parse_json_news_items(&json_data)? {
            for item in items {
                news_source.add_item(item);
            }
        }

        Ok(news_source)
    }

    async fn fetch_web_source(&self, source: &UserNewsSource) -> Result<NewsSource> {
        let client = reqwest::Client::new();
        let response = client.get(&source.url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP请求失败: {}", response.status()));
        }

        let html = response.text().await?;
        let document = scraper::Html::parse_document(&html);
        
        let mut news_source = NewsSource::new(
            source.name.clone(),
            source.title.clone(),
            source.description.clone(),
            source.url.clone(),
        );

        if let Some(selector_str) = &source.selector {
            let selector = scraper::Selector::parse(selector_str)
                .map_err(|e| anyhow!("无效的CSS选择器 '{}': {}", selector_str, e))?;

            let items = document.select(&selector);
            
            for (index, element) in items.enumerate() {
                if let Some(item) = self.parse_html_news_item(element, index)? {
                    news_source.add_item(item);
                }
            }
        }

        Ok(news_source)
    }

    fn parse_json_news_items(&self, json: &Value) -> Result<Option<Vec<NewsItem>>> {
        // 尝试多种常见的JSON新闻格式
        
        // 格式1: { "data": { "items": [...] } }
        if let Some(data) = json.get("data") {
            if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
                return Ok(Some(self.json_array_to_news_items(items)?));
            }
        }

        // 格式2: { "items": [...] }
        if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
            return Ok(Some(self.json_array_to_news_items(items)?));
        }

        // 格式3: { "news": [...] }
        if let Some(items) = json.get("news").and_then(|v| v.as_array()) {
            return Ok(Some(self.json_array_to_news_items(items)?));
        }

        // 格式4: 直接是数组
        if let Some(items) = json.as_array() {
            return Ok(Some(self.json_array_to_news_items(items)?));
        }

        warn!("无法识别的JSON格式: {}", serde_json::to_string(json).unwrap_or_default());
        Ok(None)
    }

    fn json_array_to_news_items(&self, items: &[Value]) -> Result<Vec<NewsItem>> {
        let mut news_items = Vec::new();

        for (index, item) in items.iter().enumerate() {
            let title = item.get("title")
                .or_else(|| item.get("name"))
                .or_else(|| item.get("headline"))
                .and_then(|v| v.as_str())
                .unwrap_or(&format!("新闻项 {}", index + 1))
                .to_string();

            let url = item.get("url")
                .or_else(|| item.get("link"))
                .or_else(|| item.get("permalink"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let desc = item.get("description")
                .or_else(|| item.get("summary"))
                .or_else(|| item.get("content"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let author = item.get("author")
                .or_else(|| item.get("source"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let cover = item.get("cover")
                .or_else(|| item.get("image"))
                .or_else(|| item.get("thumbnail"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let timestamp = item.get("timestamp")
                .or_else(|| item.get("published_at"))
                .or_else(|| item.get("date"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let hot = item.get("hot")
                .or_else(|| item.get("views"))
                .or_else(|| item.get("score"))
                .and_then(|v| v.as_u64());

            if !url.is_empty() {
                news_items.push(NewsItem {
                    id: format!("{}_{}", chrono::Utc::now().timestamp(), index),
                    title,
                    desc,
                    cover,
                    author,
                    timestamp,
                    hot,
                    url,
                    mobile_url: None,
                });
            }
        }

        Ok(news_items)
    }

    fn parse_html_news_item(&self, element: scraper::ElementRef, index: usize) -> Result<Option<NewsItem>> {
        // 尝试从HTML元素中提取新闻信息
        let text_collection = element.text().collect::<String>();
        let text = text_collection.trim();
        if text.is_empty() {
            return Ok(None);
        }

        // 尝试获取链接
        let url = element.value().attr("href")
            .or_else(|| element.select(&scraper::Selector::parse("a").unwrap_or_else(|_| scraper::Selector::parse("a").unwrap()))
                .next()
                .and_then(|a| a.value().attr("href")))
            .unwrap_or("")
            .to_string();

        if url.is_empty() {
            return Ok(None);
        }

        // 尝试获取图片
        let cover = element.select(&scraper::Selector::parse("img").unwrap_or_else(|_| scraper::Selector::parse("img").unwrap()))
            .next()
            .and_then(|img| img.value().attr("src"))
            .map(|s| s.to_string());

        Ok(Some(NewsItem {
            id: format!("{}_{}", chrono::Utc::now().timestamp(), index),
            title: text.to_string(),
            desc: None,
            cover,
            author: None,
            timestamp: None,
            hot: None,
            url,
            mobile_url: None,
        }))
    }
}
