import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { ApiService } from '../services/api'
import type { NewsSource, NewsSourceConfig } from '../types'

export const useNewsStore = defineStore('news', () => {
  // State
  const sources = ref<NewsSource[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Getters
  const availableSources = computed(() => sources.value)

  const isLoading = computed(() => loading.value)

  const getError = computed(() => error.value)

  // Actions
  const fetchNewsSource = async (source: string) => {
    loading.value = true
    error.value = null

    try {
      const response = await ApiService.getNews(source)
      
      if (response.success && response.data) {
        // 更新或添加到 sources 数组
        const existingIndex = sources.value.findIndex(s => s.name === source)
        if (existingIndex >= 0) {
          sources.value[existingIndex] = response.data
        } else {
          sources.value.push(response.data)
        }
      } else {
        throw new Error(response.error || '获取新闻数据失败')
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : '未知错误'
      console.error('获取新闻数据失败:', err)
    } finally {
      loading.value = false
    }
  }

  const fetchAllSources = async () => {
    const allSources: NewsSourceConfig[] = [
      { name: 'zhihu', title: '知乎', description: '有问题，就会有答案', icon: 'ChatDotRound', color: '#0084ff' },
      { name: 'weibo', title: '微博', description: '随时随地发现新鲜事', icon: 'ChatSquare', color: '#ff8200' },
      { name: 'bilibili', title: 'B站', description: '年轻人的文化社区', icon: 'VideoPlay', color: '#00a1d6' },
      { name: 'github', title: 'GitHub', description: '全球最大的代码托管平台', icon: 'Link', color: '#24292e' },
      { name: 'juejin', title: '掘金', description: '帮助开发者成长的社区', icon: 'Compass', color: '#1e80ff' },
      { name: 'douyin', title: '抖音', description: '记录美好生活', icon: 'VideoCamera', color: '#fe2c55' },
      { name: '36kr', title: '36氪', description: '科技投资媒体', icon: 'TrendCharts', color: '#00d4aa' },
      { name: 'ithome', title: 'IT之家', description: '科技媒体平台', icon: 'Monitor', color: '#ff6900' },
      { name: 'segmentfault', title: '思否', description: '技术问答社区', icon: 'QuestionFilled', color: '#00a95f' },
      { name: 'oschina', title: '开源中国', description: '开源技术社区', icon: 'Platform', color: '#0078d7' },
      { name: 'infoq', title: 'InfoQ', description: '技术媒体与社区', icon: 'Reading', color: '#2c7fb8' },
      { name: 'ruanyifeng', title: '阮一峰', description: '科技博客周刊', icon: 'Document', color: '#7b68ee' },
      { name: 'csdn', title: 'CSDN', description: 'IT技术社区', icon: 'Code', color: '#cc0000' },
      { name: 'stcn', title: '证券时报', description: '财经媒体', icon: 'TrendCharts', color: '#ff6b00' },
      { name: 'caixin', title: '财新网', description: '财经媒体', icon: 'Money', color: '#d32f2f' },
      { name: 'baidu', title: '百度', description: '百度热搜', icon: 'Search', color: '#2932e1' },
      { name: 'toutiao', title: '今日头条', description: '今日头条热点', icon: 'Notification', color: '#ff2d55' }
    ]

    loading.value = true
    error.value = null

    try {
      // 并行获取所有新闻源
      const promises = allSources.map(source => 
        ApiService.getNews(source.name).catch(err => {
          console.warn(`获取 ${source.name} 失败:`, err)
          return null
        })
      )

      const results = await Promise.all(promises)
      
      // 过滤成功的结果
      const successfulResults = results.filter((result): result is NonNullable<typeof result> => 
        result !== null && result.success === true && result.data !== undefined
      ) as Array<{ success: true; data: NewsSource }>

      sources.value = successfulResults.map(result => result.data!)
      
    } catch (err) {
      error.value = err instanceof Error ? err.message : '批量获取新闻失败'
      console.error('批量获取新闻失败:', err)
    } finally {
      loading.value = false
    }
  }

  const clearAllCache = async () => {
    try {
      await ApiService.clearCache()
      // 清空本地状态
      sources.value = []
      error.value = null
    } catch (err) {
      error.value = err instanceof Error ? err.message : '清除缓存失败'
      console.error('清除缓存失败:', err)
    }
  }

  const retryFetch = async (source: string) => {
    await fetchNewsSource(source)
  }

  const refreshNewsSource = async (source: string) => {
    // 强制刷新，不使用缓存
    await fetchNewsSource(source)
  }

  return {
    // State
    sources,
    loading,
    error,
    
    // Getters
    availableSources,
    isLoading,
    getError,
    
    // Actions
    fetchNewsSource,
    fetchAllSources,
    clearAllCache,
    retryFetch,
    refreshNewsSource
  }
})
