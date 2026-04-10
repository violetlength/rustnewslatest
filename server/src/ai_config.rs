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

impl AIConfig {
    pub async fn load() -> Result<Self> {
        let config_path = "config/ai_config.json";
        let content = fs::read_to_string(config_path)?;
        let config: AIConfig = serde_json::from_str(&content)?;
        Ok(config)
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
