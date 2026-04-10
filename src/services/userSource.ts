import { ApiService } from './api'
import type { ApiResponse } from '../types'

export interface UserNewsSource {
  id: string
  name: string
  title: string
  description: string
  source_type: 'json' | 'web'
  url: string
  selector?: string
  created_at: string
  user_id?: string
  is_active: boolean
}

export interface CreateUserSourceRequest {
  name: string
  title: string
  description: string
  source_type: 'json' | 'web'
  url: string
  selector?: string
}

class UserSourceService {
  // 获取用户数据源列表
  async getUserSources(): Promise<UserNewsSource[]> {
    try {
      const response = await ApiService.get('/api/user-sources') as ApiResponse<{user_sources: UserNewsSource[]}>
      if (response.success && response.data) {
        return response.data.user_sources
      }
      throw new Error(response.error || '获取用户数据源失败')
    } catch (error) {
      console.error('获取用户数据源失败:', error)
      throw error
    }
  }

  // 创建用户数据源
  async createUserSource(request: CreateUserSourceRequest): Promise<UserNewsSource> {
    try {
      const response = await ApiService.post('/api/user-sources', request) as ApiResponse<UserNewsSource>
      if (response.success && response.data) {
        return response.data
      }
      throw new Error(response.error || '创建用户数据源失败')
    } catch (error) {
      console.error('创建用户数据源失败:', error)
      throw error
    }
  }

  // 删除用户数据源
  async deleteUserSource(id: string): Promise<string> {
    try {
      const response = await ApiService.delete(`/api/user-sources/${id}`) as ApiResponse<string>
      if (response.success && response.data) {
        return response.data
      }
      throw new Error(response.error || '删除用户数据源失败')
    } catch (error) {
      console.error('删除用户数据源失败:', error)
      throw error
    }
  }

  // 验证数据源URL是否可访问
  async validateSourceUrl(_url: string, _sourceType: 'json' | 'web', _selector?: string): Promise<boolean> {
    try {
      // 这里可以添加验证逻辑，暂时返回true
      return true
    } catch (error) {
      console.error('验证数据源URL失败:', error)
      return false
    }
  }
}

export const UserSourceServiceInstance = new UserSourceService()
export default UserSourceServiceInstance
