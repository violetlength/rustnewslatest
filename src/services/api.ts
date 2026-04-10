import axios from 'axios'
import type { NewsSource, ApiResponse } from '../types'

const API_BASE_URL = import.meta.env?.VITE_API_BASE_URL || 'http://localhost:8080/'

class ApiServiceClass {
  private client = axios.create({
    baseURL: API_BASE_URL,
    timeout: 300000, //处理用户自定义请求AI时，等待时间长
    headers: {
      'Content-Type': 'application/json',
    },
  })

  constructor() {
    // 请求拦截器
    this.client.interceptors.request.use(
      (config) => {
        console.log(`API Request: ${config.method?.toUpperCase()} ${config.url}`)
        return config
      },
      (error) => {
        console.error('API Request Error:', error)
        return Promise.reject(error)
      }
    )

    // 响应拦截器
    this.client.interceptors.response.use(
      (response) => {
        console.log(`API Response: ${response.status} ${response.config.url}`)
        return response
      },
      (error) => {
        console.error('API Response Error:', error)
        return Promise.reject(error)
      }
    )
  }

  // 获取新闻数据
  async getNews(source: string, noCache = false): Promise<ApiResponse<NewsSource>> {
    try {
      const response = await this.client.get(`/api/news/${source}`, {
        params: { no_cache: noCache }
      })
      return response.data
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new Error(`获取${source}新闻失败: ${error.response?.data?.error || error.message}`)
      }
      throw error
    }
  }

  // 清除缓存
  async clearCache(): Promise<ApiResponse<number>> {
    try {
      const response = await this.client.delete('/api/cache')
      return response.data
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new Error(`清除缓存失败: ${error.response?.data?.error || error.message}`)
      }
      throw error
    }
  }

  // 健康检查
  async healthCheck(): Promise<ApiResponse<string>> {
    try {
      const response = await this.client.get('/api/health')
      return response.data
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new Error(`健康检查失败: ${error.response?.data?.error || error.message}`)
      }
      throw error
    }
  }

  // 图片代理
  getProxyImageUrl(originalUrl: string): string {
    return `${API_BASE_URL}/api/proxy/image?url=${encodeURIComponent(originalUrl)}`
  }

  // 通用GET请求
  async get<T = any>(url: string, config?: any): Promise<ApiResponse<T>> {
    try {
      const response = await this.client.get(url, config)
      return response.data
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new Error(`GET请求失败: ${error.response?.data?.error || error.message}`)
      }
      throw error
    }
  }

  // 通用POST请求
  async post<T = any>(url: string, data?: any, config?: any): Promise<ApiResponse<T>> {
    try {
      const response = await this.client.post(url, data, config)
      return response.data
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new Error(`POST请求失败: ${error.response?.data?.error || error.message}`)
      }
      throw error
    }
  }

  // 通用DELETE请求
  async delete<T = any>(url: string, config?: any): Promise<ApiResponse<T>> {
    try {
      const response = await this.client.delete(url, config)
      return response.data
    } catch (error) {
      if (axios.isAxiosError(error)) {
        throw new Error(`DELETE请求失败: ${error.response?.data?.error || error.message}`)
      }
      throw error
    }
  }
}

export const ApiService = new ApiServiceClass()
