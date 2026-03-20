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

use news_service::NewsService;



#[derive(Clone)]
struct AppState {
    news_service: Arc<NewsService>,
    cache: Arc<DashMap<String, CachedResponse>>,
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
        _ => Err(format!("未知的新闻源: {}", source).into()),
    };

    match result {
        Ok(news_source) => {
            let data = serde_json::to_value(&news_source).unwrap_or_default();
            
            // 缓存结果
            if !no_cache {
                let cached = CachedResponse::new(data.clone(), Duration::from_secs(300)); // 5分钟缓存
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
    <title>RustNewsLatest API</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; line-height: 1.6; }
        .header { text-align: center; margin-bottom: 40px; }
        .endpoint { background: #f8f9fa; padding: 20px; margin: 20px 0; border-radius: 8px; border-left: 4px solid #007bff; }
        .method { color: #007bff; font-weight: bold; font-family: monospace; }
        .url { color: #28a745; font-family: monospace; background: #f1f3f4; padding: 2px 6px; border-radius: 4px; }
        .description { color: #6c757d; margin: 8px 0; }
        pre { background: #f8f9fa; padding: 12px; border-radius: 4px; overflow-x: auto; }
        .news-sources { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px; margin: 20px 0; }
        .source { background: #e9ecef; padding: 10px; border-radius: 4px; text-align: center; }
    </style>
</head>
<body>
    <div class="header">
        <h1>📰 RustNewsLatest API</h1>
        <p>基于 Rust + Axum 的新闻聚合 API 服务</p>
    </div>
    
    <div class="endpoint">
        <div><span class="method">GET</span> <span class="url">/news/{source}</span></div>
        <div class="description">获取指定新闻源的数据<span style="color: #dc3545;">  (GET /news/bilibili?no_cache=true)</span></div>
    </div>
    
    <h2>📰 支持的新闻源</h2>
    <div class="news-sources">
        <div class="source">bilibili</div>
        <div class="source">weibo</div>
        <div class="source">zhihu</div>
        <div class="source">github</div>
        <div class="source">juejin</div>
        <div class="source">douyin</div>
        <div class="source">36kr</div>
        <div class="source">ithome</div>
        <div class="source">segmentfault</div>
        <div class="source">oschina</div>
        <div class="source">infoq</div>
        <div class="source">ruanyifeng</div>
        <div class="source">csdn</div>
        <div class="source">stcn</div>
        <div class="source">caixin</div>
    </div>
    
    <h2>🔗 前端应用</h2>
    <p>前端应用运行在: <a href="http://localhost:3000" target="_blank">http://localhost:3000</a></p>
</body>
</html>
    "#;
    
    Html(html)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("📰 启动 RustNewsLatest 服务器...");

    // 创建应用状态
    let news_service = Arc::new(NewsService::new());
    let app_state = AppState {
        news_service,
        cache: Arc::new(DashMap::new()),
    };

    // 创建路由
    let app = Router::new()
        // API路由
        .route("/api/health", get(health_check))
        .route("/api/news/:source", get(get_news))
        .route("/api/cache", delete(clear_cache))
        .route("/api/proxy/image", get(proxy_image))
        // 静态文件和首页
        .route("/", get(index))
        .route("/health", get(health_check))  // 兼容直接访问
        .route("/news/:source", get(get_news))  // 兼容直接访问
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
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    // let listener = TcpListener::bind("0.0.0.0:8080").await?;
    info!("🌐 服务器启动在 http://0.0.0.0:8080");
    info!("📋 API文档: http://0.0.0.0:8080");
    info!("🚀 前端应用请运行: npm run dev");

    axum::serve(listener, app).await?;

    Ok(())
}
