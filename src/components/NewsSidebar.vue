<template>
  <aside class="news-sidebar">
    <div class="sidebar-header">
      <h3>新闻源</h3>
    </div>
    <div class="sidebar-content">
      <div class="source-list">
        <div
          v-for="source in sources"
          :key="source.name"
          :class="['source-item', { active: source.name === activeSource }]"
          @click="$emit('sourceChange', source.name)"
        >
          <div class="source-icon" :style="{ color: source.color }">
            <el-icon>
              <component :is="getIconComponent(source.icon)" />
            </el-icon>
          </div>
          <div class="source-info">
            <div class="source-title">{{ source.title }}</div>
            <div class="source-desc">{{ source.description }}</div>
          </div>
        </div>
      </div>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { 
  ChatDotRound, 
  ChatSquare, 
  VideoPlay, 
  Link, 
  Compass, 
  VideoCamera, 
  TrendCharts, 
  Monitor, 
  QuestionFilled, 
  Platform, 
  Reading, 
  Document, 
  Edit,
  Money 
} from '@element-plus/icons-vue'
import type { NewsSourceConfig } from '../types'

interface Props {
  activeSource: string
  sources: NewsSourceConfig[]
}

defineProps<Props>()

defineEmits<{
  sourceChange: [source: string]
}>()

// 图标映射
const iconMap = {
  ChatDotRound,
  ChatSquare,
  VideoPlay,
  Link,
  Compass,
  VideoCamera,
  TrendCharts,
  Monitor,
  QuestionFilled,
  Platform,
  Reading,
  Document,
  Edit,
  Money
}

const getIconComponent = (iconName: string) => {
  return iconMap[iconName as keyof typeof iconMap] || Document
}
</script>

<style scoped>
.news-sidebar {
  width: 300px;
  background: white;
  border-right: 1px solid #e4e7ed;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.sidebar-header {
  padding: 1.5rem 1rem;
  border-bottom: 1px solid #e4e7ed;
}

.sidebar-header h3 {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 600;
  color: #303133;
}

.sidebar-content {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem;
}

.source-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.source-item {
  display: flex;
  align-items: center;
  padding: 0.75rem;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.3s ease;
  border: 1px solid transparent;
}

.source-item:hover {
  background-color: #f5f7fa;
  border-color: #e4e7ed;
}

.source-item.active {
  background-color: #ecf5ff;
  border-color: #409eff;
  color: #409eff;
}

.source-icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background-color: #f5f7fa;
  margin-right: 0.75rem;
  font-size: 16px;
}

.source-item.active .source-icon {
  background-color: #409eff;
  color: white;
}

.source-info {
  flex: 1;
  min-width: 0;
}

.source-title {
  font-size: 0.9rem;
  font-weight: 500;
  color: #303133;
  margin-bottom: 0.25rem;
}

.source-desc {
  font-size: 0.75rem;
  color: #909399;
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.source-item.active .source-title {
  color: #409eff;
}

.source-item.active .source-desc {
  color: #409eff;
}

@media (max-width: 768px) {
  .news-sidebar {
    width: 250px;
  }
  
  .sidebar-header {
    padding: 1rem 0.75rem;
  }
  
  .sidebar-content {
    padding: 0.25rem;
  }
  
  .source-item {
    padding: 0.5rem;
  }
  
  .source-icon {
    width: 28px;
    height: 28px;
    font-size: 14px;
    margin-right: 0.5rem;
  }
  
  .source-title {
    font-size: 0.85rem;
  }
  
  .source-desc {
    font-size: 0.7rem;
  }
}
</style>
