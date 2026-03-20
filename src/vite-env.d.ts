/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL?: string
  // 这里可以添加更多的环境变量类型定义
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
