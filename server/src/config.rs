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
    pub port: u16,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        // 尝试多个可能的配置文件位置
        let current_dir = std::env::current_dir()?;
        let config_path_absolute = current_dir.join("config.toml");
        let config_paths = [
            "config.toml",                           // 当前目录
            "../config.toml",                         // 上级目录
            "../../config.toml",                     // 上上级目录
            config_path_absolute.to_str().unwrap_or("config.toml"), // 绝对路径
        ];
        
        for config_path in &config_paths {
            if let Ok(config_content) = std::fs::read_to_string(config_path) {
                let config: Config = toml::from_str(&config_content)?;
                return Ok(config);
            }
        }
        
        Err(anyhow::anyhow!("无法找到配置文件，尝试的路径: {:?}", config_paths))
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

    pub fn get_port(&self) -> u16 {
        self.default.port
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut news_sources = HashMap::new();
        
        // 默认配置
        news_sources.insert("weibo".to_string(), 1);
        news_sources.insert("zhihu".to_string(), 30);
        news_sources.insert("bilibili".to_string(), 45);
        news_sources.insert("github".to_string(), 60);
        news_sources.insert("csdn".to_string(), 60);
        news_sources.insert("stcn".to_string(), 30);
        news_sources.insert("baidu".to_string(), 5);
        news_sources.insert("36kr".to_string(), 60);
        news_sources.insert("segmentfault".to_string(), 120);
        news_sources.insert("oschina".to_string(), 120);
        news_sources.insert("infoq".to_string(), 180);
        news_sources.insert("ruanyifeng".to_string(), 240);
        
        Self {
            news_sources,
            default: DefaultConfig {
                ttl: 60,
                http_cache_ttl: 5,
                port: 8080,
            },
        }
    }
}
