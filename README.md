# RustNewsLatest 🦀

基于 Rust + Vue3 的新闻聚合 Web 应用

## 🎯 项目简介

RustNewsLatest 是一个现代化的新闻聚合应用，采用前后端分离架构：

- **前端**: Vue3 + TypeScript + Element Plus + Pinia
- **后端**: Rust + Axum + Tokio
- **功能**: 聚合多个新闻源的数据展示

## 🏗️ 架构设计

```
┌─────────────────┐    HTTP请求    ┌─────────────────┐
│   Vue3 前端     │ ◄──────────────► │   Rust 后端     │
│                 │                │                 │
│ - 用户界面      │                │ - 数据获取      │
│ - 状态管理      │                │ - 缓存处理      │
│ - API调用       │                │ - 图片代理      │
└─────────────────┘                └─────────────────┘
```

## 📦 项目结构

```
rustNewsLatest/
├── src/                    # Vue3 前端源码
│   ├── components/         # Vue 组件
│   │   ├── NewsHeader.vue     # 新闻头部组件
│   │   ├── NewsSidebar.vue    # 侧边栏组件
│   │   ├── NewsContent.vue    # 内容显示组件
│   │   ├── NewsLayout.vue    # 布局组件
│   │   ├── UserSourceManager.vue # 自定义数据源管理
│   │   └── AIConfigModal.vue  # AI配置弹窗
│   ├── stores/            # Pinia 状态管理
│   │   └── news.ts          # 新闻状态管理
│   ├── services/          # API 服务
│   │   ├── api.ts           # API 客户端
│   │   └── userSource.ts    # 用户数据源服务
│   ├── types/             # TypeScript 类型定义
│   │   └── index.ts         # 类型定义
│   ├── views/             # Vue 页面
│   └── router/            # Vue Router 路由
├── server/                 # Rust 后端源码
│   └── src/
│       ├── main.rs        # 主程序入口和路由
│       ├── news_service.rs # 新闻服务
│       ├── ai_client.rs   # AI 客户端
│       ├── ai_config.rs   # AI 配置管理
│       ├── web_scraper.rs # 网页抓取服务
│       ├── user_source_manager.rs # 用户数据源管理
│       ├── cache.rs       # 缓存实现
│       ├── config.rs      # 配置管理
│       └── types.rs       # 数据类型定义
├── server/data/            # 数据存储目录
│   └── user_sources.json  # 用户自定义数据源
├── package.json           # 前端依赖配置
├── vite.config.ts         # Vite 构建配置
└── README.md             # 项目说明
```

## 🚀 快速开始

### 环境要求

- **Node.js**: >= 16.0.0
- **Rust**: >= 1.70.0
- **npm**: >= 8.0.0

### 安装依赖

```bash
# 安装前端依赖
npm install

# 后端依赖会在第一次编译时自动安装
```

### 启动开发服务器

#### 1. 启动 Rust 后端

```bash
# 在项目根目录
npm run server

# 或者直接运行
cd server
cargo run
```

后端服务将启动在: **http://localhost:8080**

#### 2. 启动 Vue3 前端

```bash
# 在项目根目录
npm run dev

# 或者
npm start
```

前端应用将启动在: **http://localhost:3000**

### 访问应用

- **前端应用**: http://localhost:3000
- **API文档**: http://localhost:8080
- **健康检查**: http://localhost:8080/health

## 📡 API 接口

### 获取新闻数据

```http
GET /api/news/{source}?no_cache=true
```

**内置新闻源**:
- `bilibili` - B站热门
- `weibo` - 微博热搜
- `zhihu` - 知乎热榜
- `github` - GitHub趋势
- `juejin` - 掘金热门
- `douyin` - 抖音热点
- `36kr` - 36氪
- `ithome` - IT之家
- `segmentfault` - 思否
- `oschina` - 开源中国
- `infoq` - InfoQ
- `ruanyifeng` - 阮一峰周刊
- `csdn` - CSDN
- `stcn` - 证券时报
- `caixin` - 财新网
- `baidu` - 百度热搜
- `toutiao` - 今日头条热点

**自定义数据源**:
- 支持用户添加的自定义数据源名称
- 自动识别 JSON API 和网页类型
- AI 智能解析内容结构

### 用户数据源管理

```http
GET /api/user-sources
```
获取所有用户自定义数据源

```http
POST /api/user-sources
Content-Type: application/json

{
  "name": "hellogithub",
  "title": "HelloGitHub",
  "description": "发现有趣、入门级开源项目",
  "source_type": "json",
  "url": "https://api.hellogithub.com/v1/",
  "selector": null
}
```
创建新的自定义数据源

```http
PUT /api/user-sources/{source_id}
```
更新指定数据源

```http
DELETE /api/user-sources/{source_id}
```
删除指定数据源

### AI 配置管理

```http
GET /api/ai/config
```
获取当前 AI 配置

```http
PUT /api/ai/config
Content-Type: application/json

{
  "current_config": "openai",
  "configs": {
    "openai": {
      "enabled": true,
      "provider": "openai",
      "api_key": "your-api-key",
      "base_url": "https://api.openai.com/v1",
      "model": "gpt-3.5-turbo",
      "max_tokens": 4000
    }
  }
}
```
更新 AI 配置

### 清除缓存

```http
DELETE /api/cache
```

### 图片代理

```http
GET /api/proxy/image?url={encoded_url}
```

### 健康检查

```http
GET /api/health
```

## 🎨 功能特性

### 前端功能

- ✅ **响应式设计**: 支持桌面和移动端
- ✅ **实时更新**: 自动刷新新闻数据
- ✅ **缓存管理**: 智能缓存控制
- ✅ **错误处理**: 友好的错误提示
- ✅ **加载状态**: 优雅的加载动画
- ✅ **图片代理**: 自动处理图片加载
- ✅ **自定义数据源**: 支持用户添加和管理自定义新闻源
- ✅ **AI智能解析**: 使用AI自动解析新闻内容

### 后端功能

- ✅ **高性能**: 基于 Tokio 异步运行时
- ✅ **缓存机制**: 内存缓存减少 API 调用
- ✅ **错误恢复**: 自动重试和错误处理
- ✅ **日志记录**: 完整的请求日志
- ✅ **CORS支持**: 跨域请求支持
- ✅ **图片代理**: 解决图片跨域问题

## � 自定义数据源

### 功能概述

RustNewsLatest 支持用户添加自定义新闻源，包括：
- **JSON API 数据源**: 直接从 REST API 获取结构化数据
- **网页数据源**: 从网页中通过 CSS 选择器提取新闻内容

### 使用方法

1. **打开数据源管理**
   - 点击页面右上角的"数据源管理"按钮
   - 或通过快捷键 `Ctrl+M` 打开管理面板

2. **添加自定义数据源**
   - 填写数据源信息：
     - **名称**: 唯一标识符（只能包含字母、数字、下划线和连字符）
     - **标题**: 显示名称
     - **描述**: 数据源描述
     - **类型**: 选择 `JSON API` 或 `网页`
     - **URL**: 数据源地址
     - **CSS选择器**: 仅网页类型需要

3. **JSON API 数据源示例**
   ```
   名称: hellogithub
   标题: HelloGitHub
   描述: 发现有趣、入门级开源项目
   类型: JSON API
   URL: https://api.hellogithub.com/v1/?sort_by=featured&page=1&rank_by=newest&tid=all
   ```

4. **网页数据源示例**
   ```
   名称: custom_news
   标题: 自定义新闻
   描述: 从特定网站提取新闻
   类型: 网页
   URL: https://example.com/news
   CSS选择器: .news-item
   ```

### 数据源格式要求

#### JSON API 数据源
API 应返回包含新闻项的 JSON，支持以下结构：
```json
{
  "data": [
    {
      "title": "新闻标题",
      "url": "新闻链接",
      "description": "新闻描述",
      "author": "作者",
      "published_at": "2023-01-01T00:00:00Z"
    }
  ]
}
```

#### 网页数据源
网页应包含可通过 CSS 选择器定位的新闻元素，AI 会自动分析结构并提取内容。

### AI 智能解析

系统使用 AI 自动解析未知结构的数据源：
- **智能识别**: 自动识别新闻项的标题、链接、描述等字段
- **结构分析**: 分析 JSON 或 HTML 结构，提取有用信息
- **容错处理**: 即使数据格式不标准也能尽力提取

## 🤖 AI 配置

### 支持的 AI 提供商

系统支持多种 AI 服务提供商：

- **OpenAI**: GPT-3.5, GPT-4 系列
- **DeepSeek**: DeepSeek 系列模型
- **Anthropic**: Claude 系列
- **Moonshot**: 月之暗面 Kimi
- **通义千问**: 阿里云大模型
- **百川**: 百川智能
- **豆包**: 字节跳动大模型
- **智谱AI**: GLM 系列

### 配置方法

1. **创建 AI 配置文件**
   ```bash
   # 在 server 目录下创建 ai_config.json
   touch server/ai_config.json
   ```

2. **配置文件格式**
   ```json
   {
     "current_config": "openai",
     "configs": {
       "openai": {
         "enabled": true,
         "provider": "openai",
         "api_key": "your-openai-api-key",
         "base_url": "https://api.openai.com/v1",
         "model": "gpt-3.5-turbo",
         "max_tokens": 4000
       },
       "deepseek": {
         "enabled": true,
         "provider": "deepseek",
         "api_key": "your-deepseek-api-key",
         "base_url": "https://api.deepseek.com/v1",
         "model": "deepseek-chat",
         "max_tokens": 4000
       }
     }
   }
   ```

3. **配置参数说明**
   - `enabled`: 是否启用该配置
   - `provider`: AI 提供商名称
   - `api_key`: API 密钥
   - `base_url`: API 基础地址
   - `model`: 使用的模型名称
   - `max_tokens`: 最大令牌数

### API 密钥获取

#### OpenAI
1. 访问 [OpenAI Platform](https://platform.openai.com/)
2. 注册/登录账号
3. 在 API Keys 页面创建新密钥

#### DeepSeek
1. 访问 [DeepSeek Platform](https://platform.deepseek.com/)
2. 注册/登录账号
3. 在 API Keys 页面获取密钥

#### 其他提供商
- **Anthropic**: [Anthropic Console](https://console.anthropic.com/)
- **Moonshot**: [Kimi 开放平台](https://platform.moonshot.cn/)
- **通义千问**: [阿里云百炼](https://bailian.console.aliyun.com/)
- **智谱AI**: [智谱AI开放平台](https://open.bigmodel.cn/)

### 使用 AI 功能

AI 主要用于以下场景：
- **自定义数据源解析**: 自动分析未知格式的数据源
- **网页内容提取**: 从 HTML 中智能提取新闻信息
- **规则生成**: 自动生成 CSS 选择器和解析规则
- **结构化处理**: 将非结构化数据转换为标准格式

## �� 开发指南

### 前端开发

```bash
# 开发模式
npm run dev

# 构建生产版本
npm run build

# 预览生产版本
npm run preview
```

### 后端开发

```bash
# 开发模式（自动重启）
cargo install cargo-watch
cargo watch -x run

# 构建发布版本
cargo build --release

# 运行测试
cargo test
```

### 添加新的新闻源

1. **后端**: 在 `server/src/news_service.rs` 中添加新的获取函数
2. **前端**: 在 `src/services/api.ts` 中添加对应的 API 调用
3. **配置**: 在 `src/stores/news.ts` 中添加到新闻源列表

## 📊 性能优化

### 前端优化

- **路由懒加载**: 减少初始包大小
- **组件按需加载**: 使用动态导入
- **图片懒加载**: 提升页面加载速度
- **虚拟滚动**: 处理大量数据列表

### 后端优化

- **连接池**: 复用 HTTP 连接
- **缓存策略**: 智能缓存过期时间
- **并发限制**: 防止过载
- **压缩传输**: 减少 network 开销

## 🔒 安全考虑

- **输入验证**: 严格验证所有输入参数
- **URL过滤**: 防止恶意URL访问
- **CORS配置**: 限制跨域访问源
- **错误信息**: 避免敏感信息泄露

## 🐛 故障排除

### 常见问题

1. **后端启动失败**
   ```bash
   # 检查端口是否被占用
   netstat -an | grep 8080
   
   # 更新 Rust 工具链
   rustup update
   ```

2. **前端API调用失败**
   ```bash
   # 检查后端是否运行
   curl http://localhost:8080/health
   
   # 检查代理配置
   # 确保 vite.config.ts 中代理配置正确
   ```

3. **图片加载失败**
   - 检查图片URL是否有效
   - 确认图片代理服务正常
   - 查看浏览器控制台错误信息

## 📝 待办事项

- [ ] 添加更多新闻源
- [ ] 实现用户收藏功能
- [ ] 添加搜索功能
- [ ] 支持主题切换
- [ ] 添加离线支持
- [ ] 实现推送通知


捐赠支持, 或者请我喝杯咖啡 ☕️
如果对您有帮助，请点击右上角 ⭐Star 关注或扫码捐赠，感谢支持开源！

<div style="display: flex; gap: 20px; align-items: center;">
  <img src="78b4796ba631fa985eb4ec689ef59c90.jpg" alt="捐赠二维码" width="200" />
  <img src="86b7f83f3d7708b69e6d621d29ebf2b4.png" alt="捐赠二维码" width="200" />
</div>
## 🤝 贡献指南

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情

## 🙏 致谢

- 感谢所有新闻源提供的 API 服务
- 感谢 Vue3 和 Rust 社区的贡献
- 感谢 Element Plus 的优秀组件库

---

**Made with ❤️ by violetlength**
