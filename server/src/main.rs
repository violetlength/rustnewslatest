use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response, Json},
    routing::{get, delete},
    Router,
};
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};
use types::*;

mod news_service;
mod types;
mod cache;
mod config;

use news_service::NewsService;



#[derive(Clone)]
struct AppState {
    news_service: Arc<NewsService>,
    cache: Arc<DashMap<String, CachedResponse>>,
    config: Arc<config::Config>,
}

// 缓存结构
#[derive(Clone)]
struct CachedResponse {
    data: serde_json::Value,
    timestamp: Instant,
    ttl: Duration,
}

impl CachedResponse {
    fn new(data: serde_json::Value, ttl: Duration) -> Self {
        Self {
            data,
            timestamp: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.timestamp.elapsed() > self.ttl
    }
}

// API响应结构
#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
    message: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            message: None,
        }
    }
}

// 查询参数
#[derive(Deserialize)]
struct NewsQuery {
    no_cache: Option<bool>,
}

#[derive(Deserialize)]
struct CombinedNewsQuery {
    sources: Option<String>,  // 用逗号分隔的新闻源，如 "baidu,zhihu,weibo"
    no_cache: Option<bool>,
}

#[derive(Deserialize)]
struct ImageQuery {
    url: String,
}

// 健康检查
async fn health_check() -> impl IntoResponse {
    Json(ApiResponse::success("OK"))
}

// 获取新闻数据
async fn get_news(
    State(state): State<AppState>,
    Query(query): Query<NewsQuery>,
    Path(source): Path<String>,
) -> impl IntoResponse {
    let no_cache = query.no_cache.unwrap_or(false);
    let cache_key = format!("news:{}", source);
    
    // 检查缓存
    if !no_cache {
        if let Some(cached) = state.cache.get(&cache_key) {
            if !cached.is_expired() {
                info!("返回缓存的新闻数据: {}", source);
                return Json(ApiResponse::success(cached.data.clone()));
            }
        }
    }

    // 获取新数据
    let ttl_minutes = state.config.get_ttl_for_source(&source);
    let result = match source.as_str() {
        "bilibili" => state.news_service.get_bilibili_hot(no_cache).await,
        "weibo" => state.news_service.get_weibo_hot(no_cache).await,
        "zhihu" => state.news_service.get_zhihu_hot(no_cache).await,
        "github" => state.news_service.get_github_trending(no_cache).await,
        "juejin" => state.news_service.get_juejin_hot(no_cache).await,
        "douyin" => state.news_service.get_douyin_hot(no_cache).await,
        "36kr" => state.news_service.get_36kr_hot(no_cache).await,
        "ithome" => state.news_service.get_ithome_hot(no_cache).await,
        "segmentfault" => state.news_service.get_segmentfault_hot(no_cache).await,
        "oschina" => state.news_service.get_oschina_hot(no_cache).await,
        "infoq" => state.news_service.get_infoq_hot(no_cache).await,
        "ruanyifeng" => state.news_service.get_ruanyifeng_weekly(no_cache).await,
        "csdn" => state.news_service.get_csdn_hot(no_cache).await,
        "stcn" => state.news_service.get_stcn_hot(no_cache).await,
        "caixin" => state.news_service.get_caixin_hot(no_cache).await,
        "baidu" => state.news_service.get_baidu_hot(no_cache).await,
        "toutiao" => state.news_service.get_toutiao_hot(no_cache).await,
        _ => Err(format!("未知的新闻源: {}", source).into()),
    };

    match result {
        Ok(news_source) => {
            let data = serde_json::to_value(&news_source).unwrap_or_default();
            
            // 缓存结果
            if !no_cache {
                let http_cache_ttl = state.config.get_http_cache_ttl();
                let cached = CachedResponse::new(data.clone(), Duration::from_secs(http_cache_ttl));
                state.cache.insert(cache_key, cached);
            }
            
            info!("成功获取新闻数据: {} ({} 条)", source, news_source.items.len());
            Json(ApiResponse::success(data))
        }
        Err(e) => {
            warn!("获取新闻数据失败: {} - {}", source, e);
            Json(ApiResponse::error(format!("获取{}失败: {}", source, e)))
        }
    }
}

// 获取综合新闻数据
async fn get_combined_news(
    State(state): State<AppState>,
    Query(query): Query<CombinedNewsQuery>,
) -> impl IntoResponse {
    let no_cache = query.no_cache.unwrap_or(false);
    
    // 解析新闻源，默认使用百度+知乎+微博
    let sources_str = query.sources.unwrap_or_else(|| "baidu,zhihu,weibo".to_string());
    let sources: Vec<&str> = sources_str.split(',').map(|s| s.trim()).collect();
    
    // 限制最多3个新闻源
    let limited_sources: Vec<&str> = sources.into_iter().take(3).collect();
    
    info!("获取综合新闻: {:?}", limited_sources);
    
    let mut combined_results = serde_json::Map::new();
    let mut total_items = 0;
    
    for source in &limited_sources {
        let cache_key = format!("news:{}", source);
        
        // 检查缓存
        let result = if !no_cache {
            if let Some(cached) = state.cache.get(&cache_key) {
                if !cached.is_expired() {
                    info!("返回缓存的新闻数据: {}", source);
                    Some(Ok(cached.data.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        let data = if let Some(cached_result) = result {
            cached_result
        } else {
            // 获取新数据
            fetch_news_data(&state, source, no_cache).await
        };
        
        match data {
            Ok(news_data) => {
                if let Some(items) = news_data.get("items").and_then(|v| v.as_array()) {
                    total_items += items.len();
                }
                combined_results.insert(source.to_string(), news_data);
            }
            Err(e) => {
                warn!("获取{}新闻失败: {}", source, e);
                combined_results.insert(source.to_string(), serde_json::json!({
                    "error": format!("获取{}失败: {}", source, e),
                    "items": []
                }));
            }
        }
    }
    
    // 缓存综合结果
    if !no_cache {
        let combined_key = format!("combined:{}", sources_str);
        let combined_data = serde_json::Value::Object(combined_results.clone());
        let http_cache_ttl = state.config.get_http_cache_ttl();
        let cached = CachedResponse::new(combined_data.clone(), Duration::from_secs(http_cache_ttl));
        state.cache.insert(combined_key, cached);
    }
    
    info!("成功获取综合新闻 ({} 个源, {} 条)", limited_sources.len(), total_items);
    
    Json(ApiResponse::success(serde_json::json!({
        "sources": limited_sources,
        "total_items": total_items,
        "data": combined_results
    })))
}

// 辅助函数：获取单个新闻源的数据
async fn fetch_news_data(state: &AppState, source: &str, no_cache: bool) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let result = match source {
        "bilibili" => state.news_service.get_bilibili_hot(no_cache).await,
        "weibo" => state.news_service.get_weibo_hot(no_cache).await,
        "zhihu" => state.news_service.get_zhihu_hot(no_cache).await,
        "github" => state.news_service.get_github_trending(no_cache).await,
        "juejin" => state.news_service.get_juejin_hot(no_cache).await,
        "douyin" => state.news_service.get_douyin_hot(no_cache).await,
        "36kr" => state.news_service.get_36kr_hot(no_cache).await,
        "ithome" => state.news_service.get_ithome_hot(no_cache).await,
        "segmentfault" => state.news_service.get_segmentfault_hot(no_cache).await,
        "oschina" => state.news_service.get_oschina_hot(no_cache).await,
        "infoq" => state.news_service.get_infoq_hot(no_cache).await,
        "ruanyifeng" => state.news_service.get_ruanyifeng_weekly(no_cache).await,
        "csdn" => state.news_service.get_csdn_hot(no_cache).await,
        "stcn" => state.news_service.get_stcn_hot(no_cache).await,
        "caixin" => state.news_service.get_caixin_hot(no_cache).await,
        "baidu" => state.news_service.get_baidu_hot(no_cache).await,
        "toutiao" => state.news_service.get_toutiao_hot(no_cache).await,
        _ => return Err(format!("未知的新闻源: {}", source).into()),
    };
    
    match result {
        Ok(news_source) => {
            let data = serde_json::to_value(&news_source).unwrap_or_default();
            Ok(data)
        }
        Err(e) => Err(e)
    }
}

// 清除缓存
async fn clear_cache(State(state): State<AppState>) -> impl IntoResponse {
    // 清除 HTTP 响应缓存
    let http_cache_count = state.cache.len();
    state.cache.clear();
    
    // 清除新闻服务内部的缓存
    let news_cache_count = match state.news_service.clear_cache().await {
        Ok(count) => count,
        Err(e) => {
            warn!("清除新闻服务缓存失败: {}", e);
            0
        }
    };
    
    let total_count = http_cache_count + news_cache_count;
    info!("已清除 {} 个缓存项 (HTTP缓存: {}, 新闻缓存: {})", total_count, http_cache_count, news_cache_count);
    Json(ApiResponse::success(total_count))
}

// 图片代理
async fn proxy_image(
    Query(query): Query<ImageQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("创建HTTP客户端失败: {}", e)))?;

    match client.get(&query.url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                return Err((StatusCode::BAD_GATEWAY, format!("图片获取失败: {}", response.status())));
            }

            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("image/jpeg")
                .to_string();

            match response.bytes().await {
                Ok(bytes) => {
                    info!("成功代理图片: {} ({} bytes)", query.url, bytes.len());
                    let mut resp = Response::new(axum::body::Body::from(bytes));
                    resp.headers_mut().insert(
                        header::CONTENT_TYPE,
                        header::HeaderValue::from_str(&content_type).unwrap_or_else(|_| header::HeaderValue::from_static("image/jpeg"))
                    );
                    Ok(resp)
                }
                Err(e) => {
                    warn!("读取图片数据失败: {} - {}", query.url, e);
                    return Err((StatusCode::BAD_GATEWAY, format!("读取图片失败: {}", e)));
                }
            }
        }
        Err(e) => {
            warn!("获取图片失败: {} - {}", query.url, e);
            return Err((StatusCode::BAD_GATEWAY, format!("获取图片失败: {}", e)));
        }
    }
}

// 首页
async fn index() -> impl IntoResponse {
    let html = r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>NewsLatest API</title>
    <link rel="icon" type="image/x-icon" href="/icon.ico">
    <link rel="shortcut icon" href="/icon.ico">
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; line-height: 1.6; }
        .header { text-align: center; margin-bottom: 40px; }
        .endpoint { background: #f8f9fa; padding: 20px; margin: 20px 0; border-radius: 8px; border-left: 4px solid #007bff; }
        .method { color: #007bff; font-weight: bold; font-family: monospace; }
        .url { color: #28a745; font-family: monospace; background: #f1f3f4; padding: 2px 6px; border-radius: 4px; }
        .description { color: #6c757d; margin: 8px 0; }
        pre { background: #f8f9fa; padding: 12px; border-radius: 4px; overflow-x: auto; }
        .news-sources { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px; margin: 20px 0; }
        .source { background: #e9ecef; padding: 10px; border-radius: 4px; text-align: center; cursor: pointer; transition: all 0.3s ease; }
        .source:hover { background: #007bff; color: white; transform: translateY(-2px); box-shadow: 0 4px 8px rgba(0,123,255,0.3); }
        .source a { text-decoration: none; color: inherit; display: block; }
    </style>
</head>
<body>
    <div class="header">
        <h1>📰 NewsLatest API</h1>
        <p>基于 Rust + Axum 的新闻聚合 API 服务</p>
    </div>
    
    <div class="endpoint">
        <div><span class="method">GET</span> <span class="url">/news/{source}</span></div>
        <div class="description">获取指定新闻源的数据<span style="color: #dc3545;">  (GET /news/bilibili?no_cache=false)</span></div>
    </div>
    
    <div class="endpoint">
        <div><span class="method">GET</span> <span class="url">/news/combined</span></div>
        <div class="description">获取综合新闻数据<span style="color: #dc3545;">  (GET /news/combined?sources=baidu,zhihu,weibo&no_cache=false)</span></div>
        <div class="description">支持自定义新闻源（最多3个），默认：百度+知乎+微博</div>
    </div>
    
    <h2>📰 支持的新闻源</h2>
    <div class="news-sources">
        <div class="source" style="background: #007bff; color: white;"><a href="/news/combined?no_cache=false" target="_blank" style="color: white;">🔥 综合新闻</a></div>
        <div class="source"><a href="/news/bilibili?no_cache=false" target="_blank">bilibili</a></div>
        <div class="source"><a href="/news/weibo?no_cache=false" target="_blank">weibo</a></div>
        <div class="source"><a href="/news/zhihu?no_cache=false" target="_blank">zhihu</a></div>
        <div class="source"><a href="/news/github?no_cache=false" target="_blank">github</a></div>
        <div class="source"><a href="/news/juejin?no_cache=false" target="_blank">juejin</a></div>
        <div class="source"><a href="/news/douyin?no_cache=false" target="_blank">douyin</a></div>
        <div class="source"><a href="/news/36kr?no_cache=false" target="_blank">36kr</a></div>
        <div class="source"><a href="/news/ithome?no_cache=false" target="_blank">ithome</a></div>
        <div class="source"><a href="/news/segmentfault?no_cache=false" target="_blank">segmentfault</a></div>
        <div class="source"><a href="/news/oschina?no_cache=false" target="_blank">oschina</a></div>
        <div class="source"><a href="/news/infoq?no_cache=false" target="_blank">infoq</a></div>
        <div class="source"><a href="/news/ruanyifeng?no_cache=false" target="_blank">ruanyifeng</a></div>
        <div class="source"><a href="/news/csdn?no_cache=false" target="_blank">csdn</a></div>
        <div class="source"><a href="/news/stcn?no_cache=false" target="_blank">stcn</a></div>
        <div class="source"><a href="/news/caixin?no_cache=false" target="_blank">caixin</a></div>
        <div class="source"><a href="/news/baidu?no_cache=false" target="_blank">baidu</a></div>
        <div class="source"><a href="/news/toutiao?no_cache=false" target="_blank">toutiao</a></div>
    </div>
</body>
</html>
    "#;
    
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}

// 图标服务
async fn serve_icon() -> impl IntoResponse {
    // 尝试读取图标文件
    match tokio::fs::read("icon.ico").await {
        Ok(icon_data) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, "image/x-icon")], icon_data)
        }
        Err(_) => {
            (StatusCode::NOT_FOUND, [(header::CONTENT_TYPE, "text/plain")], "Icon not found".to_string().into_bytes())
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("📰 启动 NewsLatest 服务器...");

    // 创建应用状态
    let config = match config::Config::load() {
        Ok(config) => {
            info!("✅ 配置文件加载成功");
            Arc::new(config)
        }
        Err(e) => {
            warn!("⚠️ 配置文件加载失败: {}，使用默认配置", e);
            Arc::new(config::Config::default())
        }
    };
    
    // 获取端口配置
    let config_port = config.get_port();
    
    let news_service = Arc::new(NewsService::with_config((*config).clone()));
    
    let app_state = AppState {
        news_service,
        cache: Arc::new(DashMap::new()),
        config,
    };

    // 创建路由
    let app = Router::new()
        // API路由
        .route("/api/health", get(health_check))
        .route("/api/news/:source", get(get_news))
        .route("/api/news/combined", get(get_combined_news))
        .route("/api/cache", delete(clear_cache))
        .route("/api/proxy/image", get(proxy_image))
        // 静态文件和首页
        .route("/", get(index))
        .route("/icon.ico", get(serve_icon))
        .route("/favicon.ico", get(serve_icon))
        .route("/health", get(health_check))  // 兼容直接访问
        .route("/news/:source", get(get_news))  // 兼容直接访问
        .route("/news/combined", get(get_combined_news))  // 兼容直接访问
        .route("/cache", delete(clear_cache))  // 兼容直接访问
        .route("/proxy/image", get(proxy_image))  // 兼容直接访问
        .fallback(index)
        // CORS配置
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(app_state);

    // 启动服务器
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| config_port.to_string())
        .parse()
        .unwrap_or_else(|_| config_port);
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    // let listener = TcpListener::bind("0.0.0.0:8080").await?;
    info!("🌐 服务器启动在 http://IP:{}", port);
    info!("📋 API文档: http://IP:{}", port);
    info!("🚀 前端应用请运行: npm run dev");

    axum::serve(listener, app).await?;

    Ok(())
}
