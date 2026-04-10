<template>
  <div class="news-layout">
    <NewsHeader 
      :active-source="activeSource"
      :loading="isLoading"
      :sidebar-collapsed="sidebarCollapsed"
      @refresh="handleRefresh"
      @clear-cache="handleClearCache"
      @toggle-sidebar="handleToggleSidebar"
      @show-user-sources="showUserSourceManager = true"
    />
    <div class="layout-content">
      <NewsSidebar 
        :active-source="activeSource"
        :sources="availableSources"
        :collapsed="sidebarCollapsed"
        @source-change="handleSourceChange"
      />
      <NewsContent 
        :active-source="activeSource"
        :news-source="currentNewsSource"
        :loading="isLoading"
        :error="getError || undefined"
      />
    </div>
    
    <!-- 用户数据源管理对话框 -->
    <el-dialog
      v-model="showUserSourceManager"
      title="数据源管理"
      width="90%"
      :before-close="() => showUserSourceManager = false"
      class="user-source-dialog"
    >
      <UserSourceManager />
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useNewsStore } from '../stores/news'
import NewsHeader from './NewsHeader.vue'
import NewsSidebar from './NewsSidebar.vue'
import NewsContent from './NewsContent.vue'
import UserSourceManager from './UserSourceManager.vue'
import type { NewsSource, NewsSourceConfig } from '../types'
import UserSourceService, { type UserNewsSource } from '../services/userSource'

const newsStore = useNewsStore()

const activeSource = ref('zhihu')
const sidebarCollapsed = ref(false)
const showUserSourceManager = ref(false)
const userSources = ref<UserNewsSource[]>([])

// 加载用户数据源
const loadUserSources = async () => {
  try {
    userSources.value = await UserSourceService.getUserSources()
  } catch (error) {
    console.error('加载用户数据源失败:', error)
  }
}

const availableSources = computed<NewsSourceConfig[]>(() => {
  const builtinSources = [
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
  
  const userSourcesConfig = userSources.value.map(source => ({
    name: source.name,
    title: source.title,
    description: source.description,
    icon: source.source_type === 'json' ? 'Document' : 'Monitor',
    color: source.source_type === 'json' ? '#67c23a' : '#e6a23c'
  }))
  
  return [...builtinSources, ...userSourcesConfig]
})

const currentNewsSource = computed<NewsSource | undefined>(() => 
  newsStore.availableSources.find(source => source.name === activeSource.value)
)

const isLoading = computed(() => newsStore.isLoading)
const getError = computed(() => newsStore.getError)

const handleRefresh = () => {
  newsStore.refreshNewsSource(activeSource.value)
}

const handleClearCache = async () => {
  await newsStore.clearAllCache()
  // 刷新当前数据源
  handleRefresh()
}

const handleSourceChange = (source: string) => {
  activeSource.value = source
  newsStore.fetchNewsSource(source)
}

const handleToggleSidebar = () => {
  sidebarCollapsed.value = !sidebarCollapsed.value
}

onMounted(() => {
  // 加载用户数据源
  loadUserSources()
  
  // 加载默认数据源
  newsStore.fetchNewsSource(activeSource.value)
  
  // 在移动设备上默认折叠侧边栏
  if (window.innerWidth <= 768) {
    sidebarCollapsed.value = true
  }
})
</script>

<style scoped>
.news-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #f5f5f5;
}

.layout-content {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.layout-content.sidebar-collapsed {
  grid-template-columns: auto 1fr;
}

/* 用户数据源对话框样式 */
:deep(.user-source-dialog) {
  .el-dialog__body {
    padding: 0;
    max-height: 70vh;
    overflow-y: auto;
  }
}
</style>
