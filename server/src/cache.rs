use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, SystemTime};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct CacheData {
    pub data: Value,
    pub update_time: String,
    pub expires_at_seconds: i64,
}

impl CacheData {
    pub fn new(data: Value, ttl_minutes: u64) -> Self {
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs() as i64;
        
        let ttl_seconds = ttl_minutes as i64 * 60;
        let expires_at_seconds = current_time + ttl_seconds;
        
        Self {
            data,
            update_time: Utc::now().to_rfc3339(),
            expires_at_seconds,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs() as i64;
        current_time > self.expires_at_seconds
    }
}

pub struct Cache {
    pub data: RwLock<HashMap<String, CacheData>>,
    default_ttl: Duration,
}

impl Cache {
    pub fn new(default_ttl_seconds: u64) -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            default_ttl: Duration::from_secs(default_ttl_seconds),
        }
    }

    pub async fn get(&self, key: &str) -> Option<CacheData> {
        let cache = self.data.read().await;
        cache.get(key).cloned()
    }

    pub async fn set(&self, key: String, data: Value, ttl_minutes: Option<u64>) {
        let ttl = ttl_minutes.unwrap_or((self.default_ttl.as_secs() / 60) as u64);
        let cache_data = CacheData::new(data, ttl);
        
        let mut cache = self.data.write().await;
        cache.insert(key, cache_data);
    }

    pub async fn remove(&self, key: &str) -> Option<CacheData> {
        let mut cache = self.data.write().await;
        cache.remove(key)
    }

    pub async fn clear(&self) {
        let mut cache = self.data.write().await;
        cache.clear();
    }

    pub async fn clear_expired(&self) -> usize {
        let mut cache = self.data.write().await;
        let initial_len = cache.len();
        
        cache.retain(|_, data| !data.is_expired());
        
        initial_len - cache.len()
    }

    pub async fn len(&self) -> usize {
        let cache = self.data.read().await;
        cache.len()
    }
}
