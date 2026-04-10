use reqwest::Client;
use scraper::{Html, Selector};
use crate::cache::Cache;
use crate::types::{NewsSource, NewsItem};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
type JsonArray = Vec<Value>;
use chrono::{DateTime, Utc, NaiveDate, NaiveDateTime, FixedOffset, TimeZone};
use serde_with::chrono::NaiveTime;
use crate::config::Config;
use tracing::{info, warn};
use std::time::Duration;

pub struct NewsService {
    client: Client,
    cache: Cache,
    config: Config,
}

impl NewsService {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
                .build()
                .unwrap(),
            cache: Cache::new(3600), // 1 hour default TTL
            config: Config::default(),
        }
    }

    pub fn with_config(config: Config) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
                .build()
                .unwrap(),
            cache: Cache::new(3600),
            config,
        }
    }

    pub async fn get_bilibili_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "bilibili_hot";
        // println!("bilibili_hot:{}",no_cache);
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://api.bilibili.com/x/web-interface/ranking/v2?rid=0&type=all";
        
        let response = self.client
            .get(url)
            .header("Referer", "https://www.bilibili.com/ranking/all")
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        
        let items = if let Some(data) = json.get("data").and_then(|d| d.get("list")) {
            if let Some(list) = data.as_array() {
                list.iter()
                    .take(20) // 只取前20条
                    .filter_map(|v| {
                        let bvid = v.get("bvid")?.as_str()?.to_string();
                        let title = v.get("title")?.as_str()?.to_string();
                        let desc = v.get("desc").and_then(|d| d.as_str()).map(|s| s.to_string());
                        let cover = v.get("pic")
                            .and_then(|p| p.as_str())
                            .map(|s| s.replace("http:", "https:"));
                        let author = v.get("owner")
                            .and_then(|o| o.get("name"))
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string());
                        let hot = v.get("stat")
                            .and_then(|s| s.get("view"))
                            .and_then(|v| v.as_u64());
                        let url = v.get("short_link_v2")
                            .and_then(|u| u.as_str())
                            .unwrap_or(&format!("https://www.bilibili.com/video/{}", bvid))
                            .to_string();
                        let mobile_url = Some(format!("https://m.bilibili.com/video/{}", bvid));

                        Some(NewsItem {
                            id: bvid,
                            title,
                            desc,
                            cover,
                            author,
                            timestamp: None,
                            hot,
                            url,
                            mobile_url,
                        })
                    })
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let total = items.len();
        
        // Cache the results
        // 缓存数据
        let ttl_minutes = self.config.get_ttl_for_source("bilibili");
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "bilibili".to_string(),
            title: "哔哩哔哩".to_string(),
            description: "你所热爱的，就是你的生活".to_string(),
            link: "https://www.bilibili.com/v/popular/rank/all".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    pub async fn get_weibo_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "weibo_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://weibo.com/ajax/side/hotSearch";
        
        let response = self.client
            .get(url)
            .header("Referer", "https://weibo.com/")
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        
        let items = if let Some(data) = json.get("data").and_then(|d| d.get("realtime")) {
            if let Some(list) = data.as_array() {
                list.iter().enumerate()
                    .take(20) // 只取前20条
                    .filter_map(|(index, v)| {
                        let title = v.get("word")
                            .or_else(|| v.get("word_scheme"))
                            .and_then(|w| w.as_str())
                            .unwrap_or(&format!("热搜{}", index + 1))
                            .to_string();
                        let desc = v.get("word_scheme")
                            .and_then(|w| w.as_str())
                            .map(|s| format!("#{}#", s));
                        let id = v.get("mid")
                            .or_else(|| v.get("word_scheme"))
                            .and_then(|m| m.as_str())
                            .unwrap_or(&format!("weibo-{}", index))
                            .to_string();
                        let url = format!("https://s.weibo.com/weibo?q={}", urlencoding::encode(&title));
                        let mobile_url = url.clone();

                        Some(NewsItem {
                            id,
                            title,
                            desc,
                            cover: None,
                            author: None,
                            timestamp: None,
                            hot: None,
                            url,
                            mobile_url: Some(mobile_url),
                        })
                    })
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let total = items.len();
        
        // Cache the results
        // 缓存数据
        let ttl_minutes = 1u64; // 微博1分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "weibo".to_string(),
            title: "微博".to_string(),
            description: "实时热点，每分钟更新一次".to_string(),
            link: "https://s.weibo.com/top/summary/".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    pub async fn get_zhihu_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "zhihu_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://api.zhihu.com/topstory/hot-lists/total?limit=50";
        
        let response = self.client
            .get(url)
            .header("Referer", "https://www.zhihu.com/hot")
            .header("Accept", "application/json, text/plain, */*")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        // println!("知乎API响应: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
        
        let items = if let Some(data) = json.get("data") {
            if let Some(list) = data.as_array() {
                list.iter()
                    .take(20) // 只取前20条
                    .filter_map(|v| {
                        // println!("处理条目: {}", serde_json::to_string_pretty(v).unwrap_or_default());
                        
                        // 尝试多种方式获取ID
                        let id = if let Some(target) = v.get("target") {
                            // 从target中获取ID
                            target.get("id")
                                .or_else(|| target.get("question_id"))
                                .or_else(|| target.get("article_id"))
                                .and_then(|id| {
                                    id.as_str().map(|s| s.to_string())
                                        .or_else(|| id.as_u64().map(|n| n.to_string()))
                                })
                        } else {
                            // 直接从v中获取ID
                            v.get("id")
                                .or_else(|| v.get("question_id"))
                                .or_else(|| v.get("article_id"))
                                .and_then(|id| {
                                    id.as_str().map(|s| s.to_string())
                                        .or_else(|| id.as_u64().map(|n| n.to_string()))
                                })
                        };
                        
                        let id = match id {
                            Some(id) => id,
                            None => {
                                println!("无法获取ID，跳过此条目");
                                return None;
                            }
                        };
                        
                        // 尝试多种方式获取标题
                        let title = if let Some(target) = v.get("target") {
                            target.get("title")
                                .or_else(|| target.get("question_title"))
                                .and_then(|t| t.as_str())
                        } else {
                            v.get("title")
                                .or_else(|| v.get("question_title"))
                                .and_then(|t| t.as_str())
                        };
                        
                        let title = match title {
                            Some(title) => title.to_string(),
                            None => {
                                println!("无法获取标题，使用ID作为标题");
                                id.clone()
                            }
                        };
                        
                        // 获取摘要
                        let excerpt = if let Some(target) = v.get("target") {
                            target.get("excerpt").and_then(|e| e.as_str())
                        } else {
                            v.get("excerpt").and_then(|e| e.as_str())
                        };
                        
                        let desc = excerpt.filter(|s| !s.is_empty()).map(|s| s.to_string());
                        
                        // println!("处理结果 - ID: {}, 标题: {}, 摘要: {:?}", id, title, desc);
                        // 获取热度信息
                        let hot = v.get("detail_text")
                            .and_then(|t| t.as_str())
                            .and_then(|s| {
                                // 尝试从字符串中提取数字，如 "1234 万热度"
                                s.split_whitespace()
                                    .next()
                                    .and_then(|num_str| {
                                        if num_str.contains("万") {
                                            num_str.trim_end_matches('万').parse::<f64>().ok().map(|n| (n * 10000.0) as u64)
                                        } else {
                                            num_str.parse::<u64>().ok()
                                        }
                                    })
                            });
                        
                        // println!("hot:{}",hot?.to_string());
                        let url = format!("https://www.zhihu.com/question/{}", id);
                        let mobile_url = url.clone();

                        // println!("mobile_url:{}",mobile_url);
                        Some(NewsItem {
                            id,
                            title,
                            desc,
                            cover: None,
                            author: None,
                            timestamp: None,
                            hot,
                            url,
                            mobile_url: Some(mobile_url),
                        })
                    })
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let total = items.len();
        
        // Cache the results
        // 缓存数据
        let ttl_minutes = 60u64; // 默认60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "zhihu".to_string(),
            title: "知乎".to_string(),
            description: "有问题，就会有答案".to_string(),
            link: "https://www.zhihu.com/hot".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    pub async fn get_github_trending(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "github_trending";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://api.github.com/search/repositories?q=created:>2024-01-01&sort=stars&order=desc&per_page=20";
        
        let response = self.client
            .get(url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "NewsLatest-App")
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        
        let items = if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
            items.iter()
                .take(20)
                .filter_map(|v| {
                    let id = v.get("id")?.as_u64()?.to_string();
                    let name = v.get("name")?.as_str()?.to_string();
                    let full_name = v.get("full_name")?.as_str()?.to_string();
                    let description = v.get("description")
                        .and_then(|d| d.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());
                    let stars = v.get("stargazers_count").and_then(|s| s.as_u64());
                    let language = v.get("language")
                        .and_then(|l| l.as_str())
                        .map(|s| s.to_string());
                    let html_url = v.get("html_url")?.as_str()?.to_string();
                    
                    let title = full_name.clone();
                    let desc = if let Some(ref desc) = description {
                        format!("{} | ⭐ {} | 📝 {}", desc, stars.unwrap_or(0), language.unwrap_or_else(|| "Unknown".to_string()))
                    } else {
                        format!("⭐ {} | 📝 {}", stars.unwrap_or(0), language.unwrap_or_else(|| "Unknown".to_string()))
                    };

                    Some(NewsItem {
                        id,
                        title,
                        desc: Some(desc),
                        cover: None,
                        author: Some(name),
                        timestamp: None,
                        hot: stars,
                        url: html_url.clone(),
                        mobile_url: Some(html_url),
                    })
                })
                .collect()
        } else {
            vec![]
        };

        let total = items.len();
        
        // Cache the results
        // 缓存数据
        let ttl_minutes = 60u64; // 默认60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "github".to_string(),
            title: "GitHub".to_string(),
            description: "发现全球热门开源项目".to_string(),
            link: "https://github.com/trending".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }


    pub async fn get_juejin_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "juejin_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        // 使用默认category_id = 1 (综合)，不使用limit参数，在处理时限制
        let url = "https://api.juejin.cn/content_api/v1/content/article_rank?category_id=1&type=hot";
        // println!("请求URL: {}", url);
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            // 移除Accept-Encoding以避免gzip压缩，便于调试
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("Sec-Ch-Ua", "\"Google Chrome\";v=\"123\", \"Not:A-Brand\";v=\"8\", \"Chromium\";v=\"123\"")
            .header("Sec-Ch-Ua-Mobile", "?0")
            .header("Sec-Ch-Ua-Platform", "\"Windows\"")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-User", "?1")
            .header("Upgrade-Insecure-Requests", "1")
            .header("Referer", "https://juejin.cn/")
            .send()
            .await?;

        // 检查响应状态
        let status = response.status();
        // println!("响应状态码: {}", status);
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            println!("HTTP错误 {}: {}", status, error_text);
            return Err(format!("HTTP错误 {}: {}", status, error_text).into());
        }

        // 尝试获取响应文本
        let response_text = response.text().await?;
        // println!("响应文本长度: {} 字符", response_text.len());
        
        // 如果响应是乱码（被压缩），尝试解压
        let decoded_text = if response_text.contains('\u{0}') || response_text.len() < 100 {
            println!("检测到可能的压缩响应，尝试其他方法...");
            response_text
        } else {
            response_text
        };
        
        // println!("响应前100字符: {}", &decoded_text[..decoded_text.len().min(100)]);
        
        // 尝试解析JSON
        let json: serde_json::Value = serde_json::from_str(&decoded_text)
            .map_err(|e| {
                println!("JSON解析错误: {}", e);
                e
            })?;
        // println!("JSON响应: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
        // println!("data字段: {:?}", json.get("data"));
        let items = if let Some(data) = json.get("data") {
            if let Some(list) = data.as_array() {
                list.iter()
                    .take(20)
                    .filter_map(|v| {
                        let content = v.get("content")?;
                        let author_info = v.get("author")?;
                        let content_counter = v.get("content_counter")?;
                        
                        let id = content.get("content_id")?.as_str()?.to_string();
                        let title = content.get("title")?.as_str()?.to_string();
                        let author_name = author_info.get("name")?.as_str()?.to_string();
                        let hot_rank = content_counter.get("hot_rank").and_then(|h| h.as_u64());
                        let article_url = format!("https://juejin.cn/post/{}", id);

                        Some(NewsItem {
                            id,
                            title,
                            desc: None,
                            cover: None,
                            author: Some(author_name),
                            timestamp: None,
                            hot: hot_rank,
                            url: article_url.clone(),
                            mobile_url: Some(article_url),
                        })
                    })
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let total = items.len();
        
        // Cache the results
        // 缓存数据
        let ttl_minutes = 60u64; // 默认60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "juejin".to_string(),
            title: "掘金".to_string(),
            description: "帮助开发者成长的社区".to_string(),
            link: "https://juejin.cn/hot/articles".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    pub async fn get_douyin_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "douyin_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        // 使用原项目的抖音API端点
        let url = "https://www.douyin.com/aweme/v1/web/hot/search/list/?device_platform=webapp&aid=6383&channel=channel_pc_web&detail_list=1";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Referer", "https://www.douyin.com/")
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        // println!("抖音JSON: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
        
        let items = if let Some(data) = json.get("data") {
            if let Some(list) = data.get("word_list").and_then(|l| l.as_array()) {
                list.iter()
                    .take(20)
                    .filter_map(|v| {
                        let word = v.get("word")?.as_str()?.to_string();
                        let hot_value = v.get("hot_value").and_then(|h| h.as_u64());
                        let sentence_id = v.get("sentence_id")
                            .and_then(|s| s.as_u64())
                            .unwrap_or_else(|| {
                                // 如果没有sentence_id，使用word的hash作为id
                                use std::collections::hash_map::DefaultHasher;
                                use std::hash::{Hash, Hasher};
                                let mut hasher = DefaultHasher::new();
                                word.hash(&mut hasher);
                                hasher.finish()
                            });
                        
                        let id = sentence_id.to_string();
                        let hot_url = format!("https://www.douyin.com/hot/{}", sentence_id);
                        // let search_url = format!("https://www.douyin.com/search/{}", urlencoding::encode(&word));

                        Some(NewsItem {
                            id,
                            title: word,
                            desc: None,
                            cover: None,
                            author: None,
                            timestamp: None,
                            hot: hot_value,
                            url: hot_url.clone(),
                            mobile_url: Some(hot_url),
                        })
                    }).collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let total = items.len();

        // Cache the results
        // 缓存数据
        let ttl_minutes = 60u64; // 默认60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "douyin".to_string(),
            title: "抖音".to_string(),
            description: "记录美好生活".to_string(),
            link: "https://www.douyin.com".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    pub async fn get_36kr_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "36kr_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://gateway.36kr.com/api/mis/nav/home/nav/rank/hot";
        let fallback_url = "http://gateway.36kr.com/api/mis/nav/home/nav/rank/hot";

        // 创建带有重试和超时配置的客户端
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .danger_accept_invalid_certs(false) // 不接受无效证书
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
        
        // 重试机制
        let mut retry_count = 0;
        let max_retries = 3;
        let mut use_https = true;
        
        let response: reqwest::Response = loop {
            let current_url = if use_https { url } else { fallback_url };
            
            match client
                .post(current_url)
                .header("Content-Type", "application/json; charset=utf-8")
                .header("Accept", "application/json, text/plain, */*")
                .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
                .header("Cache-Control", "no-cache")
                .header("Pragma", "no-cache")
                .json(&serde_json::json!({
                    "partner_id": "wap",
                    "param": {
                        "siteId": 1,
                        "platformId": 2
                    },
                    "timestamp": Utc::now().timestamp_millis()
                }))
                .send()
                .await
            {
                Ok(response) => break response,
                Err(e) => {
                    retry_count += 1;
                    
                    // 如果是HTTPS TLS错误，尝试HTTP
                    if use_https && retry_count == 1 && (e.to_string().contains("TLS") || e.to_string().contains("Decode")) {
                        
                        use_https = false;
                        retry_count = 0;
                        continue;
                    }
                    
                    if retry_count >= max_retries {
                        return Err(format!("请求36kr失败，已重试{}次: {}", max_retries, e).into());
                    }
                    
                    // 等待一段时间后重试
                    tokio::time::sleep(std::time::Duration::from_millis(1000 * retry_count)).await;
                }
            }
        };
        let status = response.status();
        
        // 检查响应状态
        if !status.is_success() {
            println!("响应状态码不是200: {}", status);
            return Err(format!("HTTP错误: {}", status).into());
        }

        // 先尝试获取响应文本，如果失败再尝试字节
        let items = {
            // 先尝试文本方式
            match response.text().await {
                Ok(response_text) => {
                    
                    // 尝试解析JSON
                    match serde_json::from_str::<serde_json::Value>(&response_text) {
                        Ok(json) => {
                            
                            // 根据实际JSON结构解析数据
                            if let Some(data) = json.get("data") {
                                if let Some(hot_list) = data.get("hotRankList").and_then(|l: &serde_json::Value| l.as_array()) {
                                    hot_list.iter()
                                        .take(20)
                                        .filter_map(|v: &serde_json::Value| {
                                            let item_id = v.get("itemId")?.as_u64()?;
                                            let template_material = v.get("templateMaterial")?;
                                            let title = template_material.get("widgetTitle")?.as_str()?.to_string();
                                            let cover = template_material.get("widgetImage").and_then(|img: &serde_json::Value| img.as_str()).map(|s: &str| s.to_string());
                                            let author = template_material.get("authorName").and_then(|name: &serde_json::Value| name.as_str()).map(|s: &str| s.to_string());
                                            let publish_time = v.get("publishTime").and_then(|time: &serde_json::Value| time.as_i64());
                                            // println!("publish_time: {:?}", v.get("publishTime"));
                                            let timestamp_str = if let Some(timestamp) = publish_time {
                                                //时间戳转为时间格式
                                                let timestamp_seconds = timestamp / 1000;
                                                let beijing = FixedOffset::east_opt(8 * 3600).unwrap();
                                                let beijing_time = beijing.timestamp_opt(timestamp_seconds, 0).unwrap();
                                                // println!("北京时间: {}", beijing_time);
                                                Some(beijing_time.format("%Y-%m-%d %H:%M:%S").to_string())
                                            } else {
                                                None
                                            };

                                            let stat_collect = template_material.get("statCollect").and_then(|stat: &serde_json::Value| stat.as_u64());
                                            
                                            let url = format!("https://www.36kr.com/p/{}", item_id);
                                            let mobile_url = format!("https://m.36kr.com/p/{}", item_id);

                                            Some(NewsItem {
                                                id: item_id.to_string(),
                                                title,
                                                desc: None,
                                                cover,
                                                author,
                                                timestamp: timestamp_str,
                                                hot: stat_collect,
                                                url,
                                                mobile_url: Some(mobile_url),
                                            })
                                        }).collect()
                                } else {
                                    println!("找不到hotRankList数组");
                                    vec![]
                                }
                            } else {
                                println!("找不到data字段");
                                vec![]
                            }
                        }
                        Err(e) => {
                            println!("JSON解析失败: {}", e);
                            println!("文本方式解析失败，返回空数组");
                            vec![]
                        }
                    }
                }
                Err(e) => {
                    println!("获取响应文本失败: {}", e);
                    println!("错误详情: {:?}", e);
                    vec![]
                }
            }
        };

        let total = items.len();

        // Cache the results
        let ttl_minutes = 60u64; // 默认60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "36kr".to_string(),
            title: "36氪".to_string(),
            description: "让一部分人先看到未来".to_string(),
            link: "https://36kr.com".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    // // 处理压缩响应的方法
    // async fn handle_compressed_response(&self, response: reqwest::Response) -> Result<Vec<NewsItem>, Box<dyn std::error::Error + Send + Sync>> {
    //     // 获取原始字节
    //     let bytes = response.bytes().await?;
    //     println!("获取到 {} 字节的原始数据", bytes.len());
        
    //     // 检查是否是gzip数据（gzip魔数：0x1f 0x8b）
    //     if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
    //         println!("检测到gzip压缩数据，开始解压...");
            
    //         // 尝试解压
    //         let mut decoder = GzDecoder::new(&bytes[..]);
    //         let mut decompressed = String::new();
    //         match decoder.read_to_string(&mut decompressed) {
    //             Ok(size) => {
    //                 println!("成功解压，解压后大小: {} 字符", size);
    //                 println!("解压后前100字符: {}", &decompressed[..decompressed.len().min(100)]);
                    
    //                 // 解析JSON
    //                 let json: serde_json::Value = serde_json::from_str(&decompressed)?;
    //                 return self.parse_36kr_json(json);
    //             }
    //             Err(e) => {
    //                 println!("gzip解压失败: {}", e);
    //                 return Err(Box::new(e));
    //             }
    //         }
    //     } else {
    //         println!("数据不是gzip格式，尝试直接解析");
    //         println!("前20字节: {:?}", &bytes[..bytes.len().min(20)]);
            
    //         // 尝试直接解析为字符串
    //         let text = String::from_utf8_lossy(&bytes);
    //         if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
    //             return self.parse_36kr_json(json);
    //         } else {
    //             return Err("无法解析数据".into());
    //         }
    //     }
    // }

    // 解析36kr JSON数据的辅助方法
    fn parse_36kr_json(&self, json: serde_json::Value) -> Result<Vec<NewsItem>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(data) = json.get("data") {
            if let Some(hot_list) = data.get("hotRankList").and_then(|l| l.as_array()) {
                let items: Vec<NewsItem> = hot_list.iter()
                    .take(20)
                    .filter_map(|v: &serde_json::Value| {
                        let item_id = v.get("itemId")?.as_u64()?;
                        let template_material = v.get("templateMaterial")?;
                        let title = template_material.get("widgetTitle")?.as_str()?.to_string();
                        let cover = template_material.get("widgetImage").and_then(|img: &serde_json::Value| img.as_str()).map(|s: &str| s.to_string());
                        let author = template_material.get("authorName").and_then(|name: &serde_json::Value| name.as_str()).map(|s: &str| s.to_string());
                        let publish_time = v.get("publishTime").and_then(|time: &serde_json::Value| time.as_i64());
                        let stat_collect = template_material.get("statCollect").and_then(|stat: &serde_json::Value| stat.as_u64());
                        
                        let url = format!("https://www.36kr.com/p/{}", item_id);
                        let mobile_url = format!("https://m.36kr.com/p/{}", item_id);

                        Some(NewsItem {
                            id: item_id.to_string(),
                            title,
                            desc: None,
                            cover,
                            author,
                            timestamp: publish_time.map(|t: i64| t.to_string()),
                            hot: stat_collect,
                            url,
                            mobile_url: Some(mobile_url),
                        })
                    }).collect();
                
                Ok(items)
            } else {
                Err("找不到hotRankList数组".into())
            }
        } else {
            Err("找不到data字段".into())
        }
    }

    pub async fn get_segmentfault_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "segmentfault_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://segmentfault.com/questions/hottest/weekly";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .send()
            .await?;

        let html = response.text().await?;
        // println!("思否HTML: {}", html);
        // 从HTML中提取JSON数据
        let items = self.parse_segmentfault_html(&html).unwrap_or_default();

        // println!("思否Items: {}", serde_json::to_string_pretty(&items).unwrap_or_default());
        let total = items.len();

        // Cache the results
        // 缓存数据
        let ttl_minutes = 60u64; // 默认60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "segmentfault".to_string(),
            title: "思否".to_string(),
            description: "SegmentFault 思否是中国领先的开发者社区，为开发者提供技术问答、文章分享、技术资讯等服务。".to_string(),
            link: "https://segmentfault.com".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    fn parse_segmentfault_html(&self, html: &str) -> Result<Vec<NewsItem>, Box<dyn std::error::Error + Send + Sync>> {
        // println!("开始解析SegmentFault HTML");
        
        // 使用scraper解析HTML
        use scraper::{Html, Selector};
        let document = Html::parse_document(html);
        
        // 查找__NEXT_DATA__脚本标签
        let script_selector = Selector::parse("script#__NEXT_DATA__").unwrap();
        
        if let Some(script_element) = document.select(&script_selector).next() {
            let json_content = script_element.inner_html();
            // println!("找到__NEXT_DATA__，JSON长度: {}", json_content.len());
            
            match serde_json::from_str::<serde_json::Value>(&json_content) {
                Ok(json_value) => {
                    // println!("JSON解析成功");
                    
                    // 尝试数据路径来获取热门问题
                    let data_path = vec!["props", "pageProps", "initialState", "questionList", "questionList", "rows"];
                    
                    let mut current = &json_value;
                    for key in &data_path {
                        if let Some(next) = current.get(key) {
                            current = next;
                        } else {
                            println!("路径中断在: {}", key);
                            break;
                        }
                    }
                        
                    if let Some(questions_data) = current.as_array() {
                        // println!("找到数据，数量: {}", questions_data.len());
                        let items: Vec<NewsItem> = questions_data.iter()
                            .take(20)
                            .filter_map(|v| {
                                let title = v.get("title")?.as_str()?.to_string();
                                let url_path = v.get("url")?.as_str()?;
                                let author = v.get("user")
                                    .and_then(|u| u.get("name"))
                                    .and_then(|n| n.as_str())
                                    .map(|s| s.to_string());
                                let created = v.get("created").and_then(|c| c.as_i64());
                                let timestamp_str = if let Some(timestamp) = created {
                                    //时间戳转为时间格式
                                    let beijing = FixedOffset::east_opt(8 * 3600).unwrap();
                                    let beijing_time = beijing.timestamp_opt(timestamp, 0).unwrap();
                                    // println!("北京时间: {}", beijing_time);
                                    Some(beijing_time.format("%Y-%m-%d %H:%M:%S").to_string())
                                } else {
                                    None
                                };
                                
                                let url = if url_path.starts_with("http") {
                                    url_path.to_string()
                                } else {
                                    format!("https://segmentfault.com{}", url_path)
                                };
                                
                                Some(NewsItem {
                                    id: url_path.to_string(),
                                    title,
                                    desc: None,
                                    cover: None,
                                    author,
                                    timestamp: timestamp_str,
                                    hot: Some(0),
                                    url: url.clone(),
                                    mobile_url: Some(url),
                                })
                            })
                            .collect();
                        
                        // println!("成功解析 {} 个项目", items.len());
                        return Ok(items);
                    }
                    
                    
                    // 如果上述路径都找不到，打印JSON结构以便调试
                    println!("未找到数据，JSON结构预览:");
                    if let Some(props) = json_value.get("props") {
                        println!("props键: {:?}", props.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                        if let Some(page_props) = props.get("pageProps") {
                            println!("pageProps键: {:?}", page_props.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                        }
                    }
                }
                Err(e) => {
                    println!("JSON解析失败: {}", e);
                }
            }
        } else {
            println!("未找到__NEXT_DATA__脚本标签");
        }
        
        println!("未找到任何匹配的数据");
        Ok(vec![])
    }

    pub async fn get_oschina_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "oschina_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://www.oschina.net/action/ajax/get_more_news_list?newsType=1&p=1&pageSize=20";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .send()
            .await?;

        let response_text = response.text().await?;
         println!("oschina response length: {}", response_text.len());
        
        // 解析HTML响应
        let items = self.parse_oschina_html(&response_text).unwrap_or_default();
        
        let total = items.len();
        
        // Cache the results
        // 缓存数据
        let ttl_minutes = 60u64; // 默认60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "oschina".to_string(),
            title: "开源中国".to_string(),
            description: "开源中国是目前中国最大的开源技术社区，提供最新的开源软件资讯、技术分享和开发者交流平台。".to_string(),
            link: "https://www.oschina.net".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    fn parse_oschina_html(&self, html: &str) -> Result<Vec<NewsItem>, Box<dyn std::error::Error + Send + Sync>> {
        use scraper::{Html, Selector};
        let document = Html::parse_document(html);
        
        // 查找所有新闻项
        let item_selector = Selector::parse("div.item.box").unwrap();
        let title_selector = Selector::parse("a.title span.text-ellipsis").unwrap();
        let link_selector = Selector::parse("a.title").unwrap();
        let author_selector = Selector::parse("div.from span.mr a").unwrap();
        let date_selector = Selector::parse("div.from span.mr").unwrap();
        
        let mut items = Vec::new();
        
        for element in document.select(&item_selector) {
            // 获取标题
            let title = element.select(&title_selector)
                .next()
                .and_then(|e| e.text().next())
                .unwrap_or("")
                .trim()
                .to_string();
            
            if title.is_empty() {
                continue;
            }
            
            // 获取链接
            let link = element.select(&link_selector)
                .next()
                .and_then(|e| e.value().attr("href"))
                .unwrap_or("");
            
            // 生成ID
            let id = if link.is_empty() {
                format!("oschina_{}", items.len())
            } else {
                link.to_string()
            };
            
            // 获取作者
            let author = element.select(&author_selector)
                .next()
                .and_then(|e| e.text().next())
                .map(|s| s.trim().to_string());
            
            // 获取日期 - 从span.mr中提取"发布于"后面的日期
            let date_text = element.select(&date_selector)
                .next()
                .and_then(|e| {
                    let full_text = e.text().collect::<Vec<_>>().join("").trim().to_string();
                    
                    // 查找"发布于"后面的内容
                    if let Some(pos) = full_text.find("发布于") {
                        let date_part = full_text.chars().skip(pos + 9).collect::<String>();
                        let date = date_part.trim().to_string();
                        if !date.is_empty() {
                            Some(date)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });
            
            // 构建完整URL
            let url = if link.starts_with("http") {
                link.to_string()
            } else {
                format!("https://www.oschina.net{}", link)
            };
            
            let mobile_url = if link.starts_with("http") {
                link.replace("www.oschina.net", "m.oschina.net")
            } else {
                format!("https://m.oschina.net{}", link)
            };
            
            items.push(NewsItem {
                id,
                title,
                desc: None,
                cover: None,
                author,
                timestamp: date_text,
                hot: Some(0),
                url,
                mobile_url: Some(mobile_url),
            });
            
            if items.len() >= 20 {
                break;
            }
        }
        
        println!("成功解析 {} 个开源中国新闻项", items.len());
        Ok(items)
    }

    pub async fn get_infoq_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "infoq_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://www.infoq.cn/feed";
        
        let mut response_text = String::new();
        let mut success = false;
        
        // println!("尝试InfoQ URL: {}", url);
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await?;
        
        // println!("InfoQ URL status: {}", response.status());
        
        if response.status().is_success() {
            let text = response.text().await?;
            // println!("InfoQ URL length: {}", text.len());
            
            if !text.is_empty() && text.len() > 100 {
                response_text = text;
                success = true;
                // println!("InfoQ URL 成功获取数据");
            }
        }
        
        if !success {
            println!("InfoQ URL失败，返回空数据");
            let ttl_minutes = 60u64;
            let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
            return Ok(NewsSource {
                name: "infoq".to_string(),
                title: "InfoQ".to_string(),
                description: "InfoQ是一个全球性的技术媒体平台，促进软件开发领域知识与创新的传播".to_string(),
                link: "https://www.infoq.cn/".to_string(),
                items: vec![],
                total: 0,
                from_cache: false,
                update_time: Utc::now().to_rfc3339(),
                expires_at: Some(expires_at.to_rfc3339()),
                ttl_minutes: Some(ttl_minutes),
            });
        }
        
        // println!("infoQ_response preview: {}", &response_text[..response_text.len().min(1000)]);
        
        let items = if response_text.contains("<?xml") || response_text.contains("<rss") || response_text.contains("<channel>") {
            // println!("检测到XML/RSS格式内容，使用RSS解析器");
            self.parse_infoq_rss(&response_text).unwrap_or_default()
        } else {
            println!("检测到纯HTML格式，使用RSS解析器");
            self.parse_infoq_rss(&response_text).unwrap_or_default()
        };
        
        let total = items.len();
        
        // Cache results
        let ttl_minutes = 60u64; // 默认60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "infoq".to_string(),
            title: "InfoQ".to_string(),
            description: "InfoQ是一个全球性的技术媒体平台，促进软件开发领域知识与创新的传播".to_string(),
            link: "https://www.infoq.cn/".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    fn parse_infoq_rss(&self, xml: &str) -> Result<Vec<NewsItem>, Box<dyn std::error::Error + Send + Sync>> {
        use scraper::{Html, Selector};
        let document = Html::parse_document(xml);
        
        // println!("开始解析InfoQ RSS，XML长度: {}", xml.len());
        
        // 查找RSS item
        let item_selector = Selector::parse("item").unwrap();
        let title_selector = Selector::parse("title").unwrap();
        let link_selector = Selector::parse("link").unwrap();
        // let description_selector = Selector::parse("description").unwrap();
        let author_selector = Selector::parse("author").unwrap();
        let pub_date_selector = Selector::parse("pubDate").unwrap();
        let guid_selector = Selector::parse("guid").unwrap();
        
        let mut items = Vec::new();
        // let items_found = document.select(&item_selector).count();
        // println!("找到 {} 个RSS item元素", items_found);
        
        for element in document.select(&item_selector) {
            // 获取标题
            let title = element.select(&title_selector)
                .next()
                .and_then(|e| e.text().next())
                .unwrap_or("")
                .trim()
                .to_string();
            
            // println!("标题: {}", title);
            
            if title.is_empty() {
                println!("标题为空，跳过");
                continue;
            }
            
            // 获取链接
            let link = element.select(&link_selector)
                .next()
                .and_then(|e| e.text().next())
                .unwrap_or("");
            
            // println!("链接: {}", link);
            
            // 获取作者
            let author = element.select(&author_selector)
                .next()
                .and_then(|e| e.text().next())
                .map(|s| s.trim().to_string());
            
            // println!("作者: {:?}", author);
            
            // 获取描述
            // let desc = element.select(&description_selector)
            //     .next()
            //     .and_then(|e| e.text().next())
            //     .map(|s| s.trim().to_string());
            
            // 获取发布时间并转换为北京时间
            let timestamp = element.select(&pub_date_selector)
                .next()
                .and_then(|e| e.text().next())
                .and_then(|time_str| {
                    // println!("原始时间: {}", time_str.trim());
                    // 解析GMT时间格式并转换为北京时间
                    if let Ok(datetime) = DateTime::parse_from_rfc2822(time_str.trim()) {
                        // 转换为北京时间 (UTC+8)
                        let beijing_time = datetime.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
                        let formatted_time = beijing_time.format("%Y-%m-%d %H:%M:%S").to_string();
                        // println!("转换后北京时间: {}", formatted_time);
                        Some(formatted_time)
                    } else {
                        // 如果解析失败，返回原始字符串
                        println!("时间解析失败，使用原始字符串");
                        Some(time_str.trim().to_string())
                    }
                });
            
            // println!("时间: {:?}", timestamp);
            
            // 获取GUID作为ID
            let guid = element.select(&guid_selector)
                .next()
                .and_then(|e| e.text().next())
                .unwrap_or("");
            
            // 生成ID
            let id = if guid.is_empty() {
                if link.is_empty() {
                    format!("infoq_{}", items.len())
                } else {
                    link.to_string()
                }
            } else {
                guid.to_string()
            };
            
            // 构建完整URL
            let url = if link.starts_with("http") {
                link.to_string()
            } else {
                format!("https://www.infoq.cn{}", link)
            };
            
            let mobile_url = url.clone();
            
            items.push(NewsItem {
                id,
                title,
                desc: None,
                cover: None,
                author,
                timestamp,
                hot: Some(0),
                url,
                mobile_url: Some(mobile_url),
            });
            
            if items.len() >= 20 {
                break;
            }
        }
        
        // println!("成功解析 {} 个InfoQ RSS项", items.len());
        Ok(items)
    }

    pub async fn get_ruanyifeng_weekly(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "ruanyifeng_weekly";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://www.ruanyifeng.com/blog/archives.html";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .send()
            .await?;

        let html = response.text().await?;
        let items = self.parse_ruanyifeng_html(&html).unwrap_or_default();

        let total = items.len();
        
        // Cache the results
        // 缓存数据
        let ttl_minutes = 60u64; // 默认60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "ruanyifeng".to_string(),
            title: "阮一峰博客".to_string(),
            description: "阮一峰的科技博客，包含周刊和技术文章".to_string(),
            link: "https://www.ruanyifeng.com/blog/".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    fn parse_ruanyifeng_html(&self, html: &str) -> Result<Vec<NewsItem>, Box<dyn std::error::Error + Send + Sync>> {
        use scraper::{Html, Selector};
        
        // println!("开始解析阮一峰博客HTML，长度: {}", html.len());
        let document = Html::parse_document(html);
        
        // 专门查找 id="content" 下面的 class="module-categories" 的数据
        let content_selector = Selector::parse("#alpha").unwrap();
        let module_selector = Selector::parse(".module-categories").unwrap();
        let item_selector = Selector::parse("li").unwrap();
        let title_selector = Selector::parse("a").unwrap();
        let date_selector = Selector::parse("li").unwrap(); //日期在li下获取
        let img_selector = Selector::parse("img").unwrap();
        
        let mut items = Vec::new();
        
        // 首先找到 id="content" 容器
        if let Some(content_element) = document.select(&content_selector).next() {
            // println!("找到 id=content 容器");
            
            // 在 content 容器内查找 class="module-categories" 容器
            for module_element in content_element.select(&module_selector) {
                // println!("在 content 中找到 module-categories 容器");
                
                // 在每个容器内查找 li 元素
                for (index, element) in module_element.select(&item_selector).enumerate() {
                    if index >= 20 {
                        break; // 限制最多20条
                    }
                    
                    // 获取标题和链接
                    if let Some(title_element) = element.select(&title_selector).next() {
                        let title = title_element.text().collect::<String>().trim().to_string();
                        let href = title_element.value().attr("href").unwrap_or("");
                        
                        if title.is_empty() {
                            continue;
                        }
                        
                        // 构建完整URL
                        let url = if href.starts_with("http") {
                            href.to_string()
                        } else {
                            format!("https://www.ruanyifeng.com{}", href)
                        };
                        
                        // 获取图片信息
                        let img_info = element.select(&img_selector)
                            .next()
                            .and_then(|img| {
                                let src = img.value().attr("src").unwrap_or("");
                                let alt = img.value().attr("alt").unwrap_or("");
                                if !src.is_empty() {
                                    Some((src.to_string(), alt.to_string()))
                                } else {
                                    None
                                }
                            });
                        
                        // 获取日期 - 尝试多种可能的日期元素
                        let date = element.select(&date_selector)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .filter(|s| !s.is_empty())
                            .or_else(|| {
                                // 如果没有找到专门的日期元素，尝试从li文本中提取
                                let full_text = element.text().collect::<String>();
                                // println!("完整文本内容: {}", full_text);
                                
                                // 查找可能的日期格式 (如：2026.02.14, 2026-02-28, 2026年2月28日等)
                                // 参考HTML中的图片信息来处理时间格式
                                let date_patterns = vec![
                                    r"(\d{4}\.\d{1,2}\.\d{1,2})",  // 2026.02.14 格式
                                    r"(\d{4}-\d{1,2}-\d{1,2})",    // 2026-02-28 格式
                                    r"(\d{4}年\d{1,2}月\d{1,2}日)", // 2026年2月28日 格式
                                    r"(\d{1,2}/\d{1,2}/\d{4})",    // 2/28/2026 格式
                                ];
                                
                                for pattern in date_patterns {
                                    if let Ok(re) = regex::Regex::new(pattern) {
                                        if let Some(caps) = re.captures(&full_text) {
                                            let date_str = caps.get(1).map(|m: regex::Match| m.as_str()).unwrap_or("");
                                            // println!("找到日期格式: {} -> {}", pattern, date_str);
                                            // 将 2026.02.14 格式转换为 2026-02-14 格式
                                            let normalized_date = if date_str.contains('.') {
                                                date_str.replace('.', "-")
                                            } else {
                                                date_str.to_string()
                                            };
                                            // println!("标准化日期: {}", normalized_date);
                                            return Some(normalized_date);
                                        }
                                    }
                                }
                                println!("未找到任何日期格式");
                                None
                            });
                        
                        // println!("找到文章: {} - {}", title, date.as_ref().unwrap_or(&"无日期".to_string()));
                        
                        let url_clone = url.clone();
                        items.push(NewsItem {
                            id: url_clone.clone(),
                            title,
                            desc: None,
                            cover: img_info.map(|(src, _)| {
                                if src.starts_with("http") {
                                    src
                                } else {
                                    format!("https://www.ruanyifeng.com{}", src)
                                }
                            }),
                            author: Some("阮一峰".to_string()),
                            timestamp: date,
                            hot: Some(0),
                            url: url_clone,
                            mobile_url: Some(url),
                        });
                    }
                }
            }
        } else {
            println!("未找到 id=content 容器");
        }
        
        if items.is_empty() {
            println!("未在 id=content .module-categories 中找到文章，尝试备用方案");
            // 备用方案：查找所有链接
            if let Ok(link_selector) = Selector::parse("a[href]") {
                for element in document.select(&link_selector).take(20) {
                    let title = element.text().collect::<String>().trim().to_string();
                    let href = element.value().attr("href").unwrap_or("");
                    
                    if !title.is_empty() && href.contains("blog") {
                        let url = if href.starts_with("http") {
                            href.to_string()
                        } else {
                            format!("https://www.ruanyifeng.com{}", href)
                        };
                        
                        println!("备用方案找到文章: {}", title);
                        let url_clone = url.clone();
                        items.push(NewsItem {
                            id: url_clone.clone(),
                            title,
                            desc: None,
                            cover: None,
                            author: Some("阮一峰".to_string()),
                            timestamp: None,
                            hot: Some(0),
                            url: url_clone,
                            mobile_url: Some(url),
                        });
                    }
                }
            }
        }

        // println!("成功解析 {} 个阮一峰博客文章", items.len());
        Ok(items.into_iter().take(20).collect())
    }

    pub async fn get_ithome_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "ithome_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://m.ithome.com/rankm/";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Referer", "https://www.ithome.com/")
            .send()
            .await?;

        let html = response.text().await?;
        
        // 使用简单的正则表达式解析HTML
        let items = self.parse_ithome_html(&html);

        let total = items.len();

        // Cache the results
        // 缓存数据
        let ttl_minutes = 10u64; // IT之家10分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "ithome".to_string(),
            title: "IT之家".to_string(),
            description: "爱科技，爱这里 - 前沿科技新闻网站".to_string(),
            link: "https://m.ithome.com/rankm/".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    fn parse_ithome_html(&self, html: &str) -> Vec<NewsItem> {
        let mut items = Vec::new();
        
        // 使用scraper解析HTML
        let document = Html::parse_document(html);
        
        // 选择所有包含新闻项的元素
        let news_selector = Selector::parse(".rank-box .placeholder").unwrap();
        
        for element in document.select(&news_selector) {
            // 提取标题
            let title_selector = Selector::parse(".plc-title").unwrap();
            let title = element.select(&title_selector)
                .next()
                .and_then(|el| {
                    let text = el.inner_html();
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .unwrap_or_default();
            
            // 提取链接
            let link_selector = Selector::parse("a").unwrap();
            let href = element.select(&link_selector)
                .next()
                .and_then(|el| el.value().attr("href"))
                .map(|s| s.to_string());
            
            // 提取封面图片
            let img_selector = Selector::parse("img").unwrap();
            let cover = element.select(&img_selector)
                .next()
                .and_then(|el| el.value().attr("data-original"))
                .map(|s| s.to_string());
            
            // 提取时间
            let time_selector = Selector::parse(".post-time").unwrap();
            let timestamp = element.select(&time_selector)
                .next()
                .and_then(|el| {
                    let html = el.inner_html();
                    let text = html.trim();
                    if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    }
                });
            
            // 提取热度
            let hot_selector = Selector::parse(".review-num").unwrap();
            let hot = element.select(&hot_selector)
                .next()
                .and_then(|el| {
                    let html = el.inner_html();
                    let text = html.trim();
                    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
                    digits.parse().ok()
                });
            
            if let Some(href_val) = href {
                if !title.is_empty() {
                    let id = self.extract_id_from_href(&href_val);
                    let url = self.build_ithome_url(&href_val);
                
                    items.push(NewsItem {
                        id: id.to_string(),
                        title,
                        desc: None,
                        cover,
                        author: None,
                        timestamp,
                        hot,
                        url: url.clone(),
                        mobile_url: Some(url),
                    });
                }
            }
            
            if items.len() >= 20 {
                break;
            }
        }
        
        items
    }

    fn extract_id_from_href(&self, href: &str) -> u64 {
        // 从href中提取ID，例如 /0/123/456.htm -> 123456
        use regex::Regex;
        let re = regex::Regex::new(r"/(\d+)/(\d+)\.htm").unwrap();
        if let Some(caps) = re.captures(href) {
            if let (Some(num1), Some(num2)) = (caps.get(1), caps.get(2)) {
                if let (Ok(id1), Ok(id2)) = (num1.as_str().parse::<u64>(), num2.as_str().parse::<u64>()) {
                    return id1 * 1000 + id2;
                }
            }
        }
        100000 // 默认ID
    }

    fn build_ithome_url(&self, href: &str) -> String {
        if href.starts_with("http") {
            href.to_string()
        } else {
            format!("https://www.ithome.com{}", href)
        }
    }

    pub async fn clear_cache(&self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        // 清理所有缓存数据
        let mut cache = self.cache.data.write().await;
        let count = cache.len();
        cache.clear();
        Ok(count)
    }

    pub async fn get_csdn_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "csdn_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        // println!("CSDN: 开始获取数据");
        let url = "https://blog.csdn.net/phoenix/web/blog/hot-rank?page=0&pageSize=30";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await?;

        let status = response.status();
        // println!("CSDN URL status: {}", status);
        
        if !status.is_success() {
            return Err(format!("CSDN HTTP错误: {}", status).into());
        }

        let json: serde_json::Value = response.json().await?;
        // println!("CSDN 成功获取数据");
        
        let items = if let Some(data) = json.get("data") {
            // println!("data:{}",data);
            if let Some(list) = data.as_array() {
                list.iter().take(20).filter_map(|v| {
                    let product_id = v.get("productId")?.as_str()?.to_string();
                    let article_title = v.get("articleTitle")?.as_str()?.to_string();
                    let nick_name = v.get("nickName")?.as_str()?.to_string();
                    let article_detail_url = v.get("articleDetailUrl")?.as_str()?.to_string();
                    let hot_rank_score = v.get("hotRankScore")?.as_str().unwrap_or("0").to_string();
                    let period = v.get("period")?.as_str().unwrap_or("0").to_string();
                    
                    // 获取封面图
                    let cover = v.get("picList")
                        .and_then(|pics| pics.as_array())
                        .and_then(|pics| pics.first())
                        .and_then(|pic| pic.as_str())
                        .map(|s| s.to_string());
                    
                    // 处理时间 - period格式为"年-月-日-时"，如"2026-03-02-08"
                    let timestamp = if !period.is_empty() && period.contains('-') {
                        // 将"2026-03-02-08"转换为"2026-03-02 08:00:00"
                        let parts: Vec<&str> = period.split('-').collect();
                        if parts.len() == 4 {
                            if let (Ok(year), Ok(month), Ok(day), Ok(hour)) = (
                                parts[0].parse::<i32>(),
                                parts[1].parse::<u32>(),
                                parts[2].parse::<u32>(),
                                parts[3].parse::<u32>()
                            ) {
                                let datetime = NaiveDateTime::new(
                                    NaiveDate::from_ymd_opt(year, month, day)?,
                                    NaiveTime::from_hms_opt(hour, 0, 0)?
                                );
                                Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    
                    Some(NewsItem {
                        id: product_id.clone(),
                        title: article_title,
                        desc: None,
                        cover,
                        author: Some(nick_name),
                        timestamp,
                        hot: hot_rank_score.parse().ok(),
                        url: article_detail_url.clone(),
                        mobile_url: Some(article_detail_url),
                    })
                }).collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        // 缓存数据
        let ttl_minutes = 60u64; // 默认60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "csdn".to_string(),
            title: "CSDN".to_string(),
            description: "专业开发者社区".to_string(),
            link: "https://www.csdn.net/".to_string(),
            items: items.clone(),
            total: items.len(),
            from_cache: false,
            update_time: chrono::Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;
        
        Ok(news_source)
    }

    pub async fn get_stcn_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "stcn_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://www.stcn.com/article/list/yw.html";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await?;

        let html = response.text().await?;
        // println!("stcn:{}",&html);
        let items = self.parse_stcn_html(&html).unwrap_or_default();

        let total = items.len();
        
        // 缓存数据
        let ttl_minutes = 30u64; // 证券时报30分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "stcn".to_string(),
            title: "证券时报".to_string(),
            description: "证券时报网要闻频道，提供最新财经要闻".to_string(),
            link: "https://www.stcn.com/article/list/yw.html".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    fn parse_stcn_html(&self, html: &str) -> Result<Vec<NewsItem>, Box<dyn std::error::Error + Send + Sync>> {
        use scraper::{Html, Selector};
        let document = Html::parse_document(html);
        
        // 查找新闻列表项 - 根据实际页面结构更新选择器
        let news_selector = Selector::parse(".list-page-tab-content.active .list.infinite-list li").unwrap();
        let elements: Vec<_> = document.select(&news_selector).collect();
        // println!("STCN: 找到 {} 个新闻元素", elements.len());
        
        let mut items = Vec::new();
        
        for (index, element) in elements.iter().enumerate() {
            // 提取标题和链接 - 从a标签中获取
            let link_selector = Selector::parse("a").unwrap();
            let link_element = element.select(&link_selector).next();
            
            let title: String;
            let url: String;
            
            if let Some(a_element) = link_element {
                title = a_element.text().collect::<String>().trim().to_string();
                url = a_element.value().attr("href")
                    .map(|s| {
                        if s.starts_with("http") {
                            s.to_string()
                        } else {
                            format!("https://www.stcn.com{}", s)
                        }
                    })
                    .unwrap_or_else(|| "https://www.stcn.com".to_string());
                
                // println!("STCN: 第{}条 - 标题: {}", index + 1, title);
            } else {
                println!("STCN: 第{}个元素没有找到链接", index + 1);
                continue;
            }
            
            if title.is_empty() {
                println!("STCN: 第{}条标题为空，跳过", index + 1);
                continue;
            }
            
            // 提取时间 - 从span或其他时间元素中获取
            let time_selector = Selector::parse(".time, .date, .publish-time, span").unwrap();
            let timestamp = element.select(&time_selector)
                .next()
                .and_then(|el| {
                    let text = el.text().collect::<String>();
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                });
            
            // 提取作者/来源
            let author_selector = Selector::parse(".author, .source, .media").unwrap();
            let author = element.select(&author_selector)
                .next()
                .and_then(|el| {
                    let text = el.text().collect::<String>();
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                });
            
            // 提取描述
            let desc_selector = Selector::parse(".desc, .summary, p").unwrap();
            let desc = element.select(&desc_selector)
                .next()
                .and_then(|el| {
                    let text = el.text().collect::<String>();
                    let trimmed = text.trim();
                    if trimmed.is_empty() || trimmed.len() > 200 {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                });
            
            items.push(NewsItem {
                id: format!("stcn_{}", items.len() + 1),
                title,
                desc,
                cover: None,
                author,
                timestamp,
                hot: None,
                url: url.clone(),
                mobile_url: Some(url),
            });
            
            // 限制50条
            if items.len() >= 50 {
                break;
            }
        }
        
        // println!("STCN: 最终解析出 {} 条新闻", items.len());
        Ok(items)
    }

    pub async fn get_caixin_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "caixin_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://finance.caixin.com/";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await?;

        let html = response.text().await?;
        let items = self.parse_caixin_html(&html).unwrap_or_default();

        let total = items.len();
        
        // 缓存数据
        let ttl_minutes = 60u64; // 财新网60分钟TTL
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "caixin".to_string(),
            title: "财新网".to_string(),
            description: "财新网金融频道，提供专业财经新闻".to_string(),
            link: "https://finance.caixin.com/".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    fn parse_caixin_html(&self, html: &str) -> Result<Vec<NewsItem>, Box<dyn std::error::Error + Send + Sync>> {
        use scraper::{Html, Selector};
        let document = Html::parse_document(html);
        
        // 查找id为listArticle的容器
        let list_selector = Selector::parse("#listArticle").unwrap();
        let list_container = document.select(&list_selector).next();
        
        if list_container.is_none() {
            return Ok(Vec::new());
        }
        
        // 在listArticle容器内查找class为boxa的新闻项
        let news_selector = Selector::parse(".boxa").unwrap();
        let mut items = Vec::new();
        let mut seen_titles = std::collections::HashSet::new();
        
        for element in list_container.unwrap().select(&news_selector) {
            // 提取标题 - 从h4 a中获取
            let title_selector = Selector::parse("h4 a").unwrap();
            let title = element.select(&title_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            
            if title.is_empty() || seen_titles.contains(&title) {
                continue;
            }
            
            seen_titles.insert(title.clone());
            
            // 提取链接 - 从h4 a中获取
            let href = element.select(&title_selector)
                .next()
                .and_then(|el| el.value().attr("href"))
                .map(|s| {
                    if s.starts_with("http") {
                        s.to_string()
                    } else {
                        format!("https://finance.caixin.com{}", s)
                    }
                })
                .unwrap_or_else(|| "https://finance.caixin.com/".to_string());
            
            // 提取封面图片 - 从.pic img中获取
            let cover = element.select(&Selector::parse(".pic img").unwrap())
                .next()
                .and_then(|el| {
                    el.value().attr("data-src")
                        .or_else(|| el.value().attr("src"))
                        .map(|s| s.to_string())
                });
            
            // 提取时间戳和作者 - 从span中获取
            let timestamp_author = element.select(&Selector::parse("span").unwrap())
                .next()
                .and_then(|el| {
                    let text = el.text().collect::<String>();
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                });
            
            // 分离时间戳和作者
            let (timestamp, author) = if let Some(ts_auth) = timestamp_author {
                // 格式通常是: "文｜财新 张宇哲 2026年03月03日 20:17"
                if let Some(pipe_pos) = ts_auth.find('｜') {
                    let author_part = ts_auth[..pipe_pos].trim();
                    let time_part = ts_auth[pipe_pos + 3..].trim(); // 跳过"｜"
                    (Some(time_part.to_string()), Some(author_part.to_string()))
                } else {
                    (Some(ts_auth.clone()), Some("财新网".to_string()))
                }
            } else {
                (None, Some("财新网".to_string()))
            };
            
            // 提取描述 - 从p标签中获取
            let desc_selector = Selector::parse("p").unwrap();
            let desc_elements: Vec<_> = element.select(&desc_selector).collect();
            let desc = if desc_elements.is_empty() {
                None
            } else {
                let p_text = desc_elements[0].text().collect::<String>();
                let trimmed = p_text.trim();
                
                if trimmed.is_empty() {
                    None
                } else if trimmed.len() > 500 { // 增加限制到500字符
                    // 截取前500字符而不是完全忽略
                    Some(trimmed.chars().take(500).collect::<String>() + "...")
                } else {
                    Some(trimmed.to_string())
                }
            };
            
            items.push(NewsItem {
                id: format!("caixin_{}", items.len() + 1),
                title,
                desc,
                cover,
                author,
                timestamp,
                hot: None,
                url: href.clone(),
                mobile_url: Some(href),
            });
            
            // 限制50条
            if items.len() >= 50 {
                break;
            }
        }
        
        Ok(items)
    }

    pub async fn get_baidu_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "baidu_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        let url = "https://zj.v.api.aa1.cn/api/baidu-rs/";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await?;

        let json_text = response.text().await?;
        
        // 
        info!(": {} ", json_text.len());
        
        // 
        let safe_end = json_text.len().min(200);
        let safe_truncated = if safe_end < json_text.len() {
            // 
            let mut end = safe_end;
            while end > 0 && !json_text.is_char_boundary(end) {
                end -= 1;
            }
            &json_text[..end]
        } else {
            &json_text
        };
        info!(": {}", safe_truncated);
        
        // JSON
        let json_data: serde_json::Value = serde_json::from_str(&json_text)
            .map_err(|e| {
                warn!(": {}, : {}", e, safe_truncated);
                e
            })?;
        
        // data
        let data_value = json_data.get("data");
        if let Some(data) = data_value {
            info!("成功获取data字段，类型: {:?}", data);
            
            // 检查data是否为数组
            if let Some(data_array) = data.as_array() {
                info!("百度热搜data数组长度: {}", data_array.len());
                
                // 验证数组不为空
                if data_array.is_empty() {
                    warn!("百度热搜data数组为空");
                    return self.create_empty_baidu_result();
                }
                
                // 直接使用data数组创建NewsItem
                let items = self.parse_baidu_data_array(data_array)?;
                
                let total = items.len();
                
                // 缓存数据
                let ttl_minutes = self.config.get_ttl_for_source("baidu");
                let expires_at = chrono::Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
                
                let news_source = NewsSource {
                    name: "baidu".to_string(),
                    title: "百度热搜".to_string(),
                    description: "百度实时热搜榜单，反映当前最热门的搜索关键词".to_string(),
                    link: "https://top.baidu.com/board?tab=realtime".to_string(),
                    items,
                    total,
                    from_cache: false,
                    update_time: Utc::now().to_rfc3339(),
                    expires_at: Some(expires_at.to_rfc3339()),
                    ttl_minutes: Some(ttl_minutes),
                };
                
                info!("成功解析百度热搜数据，共 {} 条记录", total);
                return Ok(news_source);
            } else {
                warn!("百度热搜data字段不是数组类型: {:?}", data);
                return self.create_empty_baidu_result();
            }
        } else {
            warn!("百度热搜JSON中缺少data字段");
            return self.create_empty_baidu_result();
        }
    }

    fn create_empty_baidu_result(&self) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let news_source = NewsSource {
            name: "baidu".to_string(),
            title: "百度热搜".to_string(),
            description: "百度实时热搜榜单，反映当前最热门的搜索关键词".to_string(),
            link: "https://top.baidu.com/board?tab=realtime".to_string(),
            items: vec![],
            total: 0,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: None,
            ttl_minutes: Some(5),
        };
        Ok(news_source)
    }

    fn parse_baidu_data_array(&self, data_array: &JsonArray) -> Result<Vec<NewsItem>, Box<dyn std::error::Error + Send + Sync>> {
        let mut items = Vec::new();
        
        // 只取前20条记录
        for (index, item) in data_array.iter().take(20).enumerate() {
            // 使用最简单的方式提取字段
            let title: String = item.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown_title")
                .to_string();
                
            let url = item.get("url")
                .and_then(|v: &Value| v.as_str())
                .unwrap_or("")
                .to_string();
                
            let hot_str = item.get("hot")
                .and_then(|v: &Value| v.as_str())
                .unwrap_or("0");
            let hot = hot_str.parse::<u64>().unwrap_or(0);
            
            let desc = item.get("desc")
                .and_then(|v: &Value| v.as_str())
                .unwrap_or("")
                .to_string();
                
            let pic = item.get("pic")
                .and_then(|v: &Value| v.as_str())
                .unwrap_or("")
                .to_string();
            
            // 只有当URL不为空时才添加新闻项
            if !url.is_empty() {
                let news_item = NewsItem {
                    id: format!("{}", index + 1),
                    title,
                    desc: Some(desc),
                    cover: if pic.is_empty() { None } else { Some(pic) },
                    author: Some("百度热搜".to_string()),
                    timestamp: Some(Utc::now().to_rfc3339()),
                    hot: Some(hot),
                    url: url.clone(),
                    mobile_url: Some(url),
                };
                
                items.push(news_item);
            }
        }
        
        info!("成功解析百度热搜数组，共 {} 条记录", items.len());
        Ok(items)
    }

    pub async fn get_toutiao_hot(&self, no_cache: bool) -> Result<NewsSource, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = "toutiao_hot";
        
        if !no_cache {
            if let Some(cached_data) = self.cache.get(cache_key).await {
                let news_source: NewsSource = serde_json::from_value(cached_data.data)?;
                return Ok(news_source);
            }
        }

        // 今日头条热榜API
        let url = "https://www.toutiao.com/hot-event/hot-board/?origin=toutiao_pc";
        
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .header("Referer", "https://www.toutiao.com/")
            .send()
            .await?;

        let response_text = response.text().await?;
        info!("今日头条API返回数据长度: {} 字符", response_text.len());
        
        // 解析JSON响应
        let json: serde_json::Value = serde_json::from_str(&response_text)?;
        let mut items = Vec::new();
        
        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
            for (index, item) in data.iter().enumerate() {
                let title = item.get("Title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                
                if title.is_empty() {
                    continue;
                }
                
                let url = item.get("Url")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let hot = item.get("HotValue")
                    .and_then(|h| h.as_u64())
                    .unwrap_or(0);
                
                let desc = item.get("Abstract")
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();
                
                // 构建完整URL
                let full_url = if url.starts_with("http") {
                    url.to_string()
                } else if url.starts_with("/") {
                    format!("https://www.toutiao.com{}", url)
                } else {
                    format!("https://www.toutiao.com/{}", url)
                };
                
                items.push(NewsItem {
                    id: format!("toutiao_{}", index + 1),
                    title,
                    desc: if desc.is_empty() { None } else { Some(desc) },
                    cover: None,
                    author: Some("今日头条".to_string()),
                    timestamp: Some(Utc::now().to_rfc3339()),
                    hot: Some(hot),
                    url: full_url.clone(),
                    mobile_url: Some(url),
                });
                
                if items.len() >= 20 {
                    break;
                }
            }
        } else {
            warn!("今日头条JSON格式不正确，缺少data字段");
        }
        
        let total = items.len();
        
        // 缓存数据
        let ttl_minutes = 60u64; // 60分钟TTL
        let expires_at = Utc::now() + chrono::Duration::minutes(ttl_minutes as i64);
        
        let news_source = NewsSource {
            name: "toutiao".to_string(),
            title: "今日头条".to_string(),
            description: "今日头条热榜，提供最新最热的新闻资讯和热点事件".to_string(),
            link: "https://www.toutiao.com".to_string(),
            items: items.clone(),
            total,
            from_cache: false,
            update_time: Utc::now().to_rfc3339(),
            expires_at: Some(expires_at.to_rfc3339()),
            ttl_minutes: Some(ttl_minutes),
        };

        self.cache.set(cache_key.to_string(), serde_json::to_value(&news_source)?, Some(ttl_minutes * 60)).await;

        Ok(news_source)
    }

    fn parse_toutiao_html(&self, html: &str) -> Result<Vec<NewsItem>, Box<dyn std::error::Error + Send + Sync>> {
        use scraper::{Html, Selector};
        let document = Html::parse_document(html);
        
        // 查找 ttp-hot-board 下面的 ol 元素
        let hot_board_selector = Selector::parse(".ttp-hot-board").unwrap();
        let ol_selector = Selector::parse("ol").unwrap();
        let li_selector = Selector::parse("li").unwrap();
        let title_selector = Selector::parse("a").unwrap();
        
        let mut items = Vec::new();
        
        // 首先找到热榜容器
        if let Some(hot_board) = document.select(&hot_board_selector).next() {
            // 在容器内查找 ol 元素
            if let Some(ol_element) = hot_board.select(&ol_selector).next() {
                // 遍历 ol 下的 li 元素
                for (index, li_element) in ol_element.select(&li_selector).enumerate() {
                    // 获取标题和链接
                    let title_and_url = li_element.select(&title_selector)
                        .next()
                        .and_then(|a| {
                            let title = a.text().collect::<Vec<_>>().join("").trim().to_string();
                            let url = a.value().attr("href").unwrap_or("").to_string();
                            if !title.is_empty() {
                                Some((title, url))
                            } else {
                                None
                            }
                        });
                    
                    if let Some((title, url)) = title_and_url {
                        // 构建完整URL
                        let full_url = if url.starts_with("http") {
                            url.to_string()
                        } else if url.starts_with("/") {
                            format!("https://www.toutiao.com{}", url)
                        } else {
                            format!("https://www.toutiao.com/{}", url)
                        };
                        
                        items.push(NewsItem {
                            id: format!("toutiao_{}", index + 1),
                            title,
                            desc: None,
                            cover: None,
                            author: Some("今日头条".to_string()),
                            timestamp: Some(Utc::now().to_rfc3339()),
                            hot: Some(0),
                            url: full_url.clone(),
                            mobile_url: Some(full_url.replace("www.toutiao.com", "m.toutiao.com")),
                        });
                        
                        if items.len() >= 20 {
                            break;
                        }
                    }
                }
            }
        }
        
        info!("成功解析 {} 个今日头条新闻项", items.len());
        Ok(items)
    }

    

}
