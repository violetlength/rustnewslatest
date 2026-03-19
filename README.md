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
│   ├── stores/            # Pinia 状态管理
│   ├── services/          # API 服务
│   ├── types/             # TypeScript 类型定义
│   ├── views/             # Vue 页面
│   └── router/            # Vue Router 路由
├── server/                 # Rust 后端源码
│   └── src/
│       ├── main.rs        # 主程序入口
│       ├── news_service.rs # 新闻服务
│       └── types.rs       # 数据类型定义
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

**支持的新闻源**:
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

### 后端功能

- ✅ **高性能**: 基于 Tokio 异步运行时
- ✅ **缓存机制**: 内存缓存减少 API 调用
- ✅ **错误恢复**: 自动重试和错误处理
- ✅ **日志记录**: 完整的请求日志
- ✅ **CORS支持**: 跨域请求支持
- ✅ **图片代理**: 解决图片跨域问题

## 🔧 开发指南

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

**Made with ❤️ by RustNewsLatest Team**
