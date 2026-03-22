use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub news_sources: HashMap<String, u64>,
    pub default: DefaultConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefaultConfig {
    pub ttl: u64,
    pub http_cache_ttl: u64,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_content = std::fs::read_to_string("config.toml")?;
        let config: Config = toml::from_str(&config_content)?;
        Ok(config)
    }

    pub fn get_ttl_for_source(&self, source: &str) -> u64 {
        self.news_sources
            .get(source)
            .copied()
            .unwrap_or(self.default.ttl)
    }

    pub fn get_http_cache_ttl(&self) -> u64 {
        self.default.http_cache_ttl
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut news_sources = HashMap::new();
        
        // 默认配置
        news_sources.insert("weibo".to_string(), 1);
        news_sources.insert("zhihu".to_string(), 30);
        news_sources.insert("bilibili".to_string(), 45);
        news_sources.insert("github".to_string(), 120);
        news_sources.insert("juejin".to_string(), 60);
        news_sources.insert("douyin".to_string(), 5);
        news_sources.insert("36kr".to_string(), 90);
        news_sources.insert("ithome".to_string(), 60);
        news_sources.insert("segmentfault".to_string(), 60);
        news_sources.insert("oschina".to_string(), 60);
        news_sources.insert("infoq".to_string(), 90);
        news_sources.insert("ruanyifeng".to_string(), 180);
        news_sources.insert("csdn".to_string(), 60);
        news_sources.insert("stcn".to_string(), 30);
        news_sources.insert("caixin".to_string(), 60);

        Self {
            news_sources,
            default: DefaultConfig {
                ttl: 60,
                http_cache_ttl: 300,
            },
        }
    }
}
