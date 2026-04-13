use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub current_config: CurrentAIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentAIConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub api_base: Option<String>,
    pub enabled: bool,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            current_config: CurrentAIConfig {
                provider: "deepseek".to_string(),
                api_key: String::new(),
                model: "deepseek-chat".to_string(),
                api_base: None,
                enabled: false,
            }
        }
    }
}

impl AIConfig {
    pub async fn load() -> Result<Self> {
        let config_path = "config/ai_config.json";
        
        // 检查配置文件是否存在
        if !std::path::Path::new(config_path).exists() {
            // 创建 config 目录
            if let Some(parent) = std::path::Path::new(config_path).parent() {
                fs::create_dir_all(parent)?;
            }
            // 创建默认配置文件
            let default_config = Self::default();
            let content = serde_json::to_string_pretty(&default_config)?;
            fs::write(config_path, content)?;
            return Ok(default_config);
        }
        
        let content = fs::read_to_string(config_path)?;
        let config: AIConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    pub fn save(&self) -> Result<()> {
        let config_path = "config/ai_config.json";
        
        // 确保 config 目录存在
        if let Some(parent) = std::path::Path::new(config_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIRequest {
    model: String,
    messages: Vec<AIMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct AIResponse {
    pub choices: Vec<AIChoice>,
}

#[derive(Debug, Deserialize)]
pub struct AIChoice {
    pub message: AIMessage,
}

#[derive(Debug, Deserialize)]
pub struct ParsedNewsItem {
    pub title: String,
    pub url: String,
    pub desc: Option<String>,
    pub timestamp: Option<String>,
    pub author: Option<String>,
    pub cover: Option<String>,
    pub hot: Option<String>,
}
