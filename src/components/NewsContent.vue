<template>
  <main class="news-content">
    <div class="content-header" v-if="newsSource">
      <div class="header-info">
        <h2>{{ newsSource.title }}</h2>
        <p>{{ newsSource.description }}</p>
        <div class="header-meta">
          <span class="update-time">更新时间: {{ formattedUpdateTime }}</span>
          <span class="item-count">共 {{ newsSource.items.length }} 条</span>
        </div>
      </div>
    </div>

    <div class="content-body" ref="contentBodyRef">
      <div v-if="loading" class="loading-container">
        <el-icon class="loading-icon"><Loading /></el-icon>
        <p>正在加载新闻数据...</p>
      </div>

      <div v-else-if="error" class="error-container">
        <el-icon class="error-icon"><Warning /></el-icon>
        <p>{{ error }}</p>
      </div>

      <div v-else-if="!newsSource || newsSource.items.length === 0" class="empty-container">
        <el-icon class="empty-icon"><Document /></el-icon>
        <p>暂无新闻数据</p>
      </div>

      <div v-else class="news-list">
        <div
          v-for="item in newsSource.items"
          :key="item.id"
          class="news-item"
          @click="openUrlDrawer(item.url)"
        >
          <div class="news-cover" v-if="item.cover">
            <img
              :src="item.cover"
              :alt="item.title"
              @error="handleImageError($event, item.cover)"
            />
          </div>
          <div class="news-main">
            <h3 class="news-title">{{ item.title }}</h3>
            <p class="news-desc" v-if="item.desc">{{ item.desc }}</p>
            <div class="news-meta">
              <span class="news-author" v-if="item.author">{{ item.author }}</span>
              <span class="news-hot" v-if="item.hot">
                <el-icon><Star /></el-icon>
                {{ formatHotCount(item.hot) }}
              </span>
              <span class="news-time" v-if="item.timestamp">{{ item.timestamp }}</span>
            </div>
          </div>
          <div class="news-action">
            <el-icon><Link /></el-icon>
          </div>
        </div>
      </div>
    </div>
  </main>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElMessageBox } from 'element-plus'
import { Loading, Warning, Document, Star, Link } from '@element-plus/icons-vue'
import type { NewsSource } from '../types'

interface Props {
  activeSource: string
  newsSource?: NewsSource
  loading: boolean
  error?: string
}

const props = defineProps<Props>()

// 滚动容器引用
const contentBodyRef = ref<HTMLElement>()

const formattedUpdateTime = computed(() => {
  if (!props.newsSource?.update_time) return "";
  const date = new Date(props.newsSource.update_time);
  return date.toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
});

// 格式化热度数字
function formatHotCount(hot?: number): string {
  if (!hot) return "";
  if (hot >= 10000) {
    return (hot / 10000).toFixed(1) + "万";
  }
  return hot.toString();
}

// 处理图片加载错误
async function handleImageError(event: Event, originalUrl: string) {
  const img = event.target as HTMLImageElement;
  if (!img) return;
  
  try {
    // 使用后端代理接口
    const proxyUrl = `/api/proxy/image?url=${encodeURIComponent(originalUrl)}`;
    img.src = proxyUrl;
    
    // 如果代理也失败，设置默认图片
    img.onerror = () => {
      img.src = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgZmlsbD0iI2Y0ZjRmNCIvPjx0ZXh0IHg9IjUwIiB5PSI1MCIgZm9udC1mYW1pbHk9IkFyaWFsIiBmb250LXNpemU9IjEyIiBmaWxsPSIjOTk5IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBkeT0iLjNlbSI+5Zu+54mHPC90ZXh0Pjwvc3ZnPg==";
    };
  } catch (error) {
    console.error("Failed to proxy image:", error);
    // 设置默认图片
    img.src = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgZmlsbD0iI2Y0ZjRmNCIvPjx0ZXh0IHg9IjUwIiB5PSI1MCIgZm9udC1mYW1pbHk9IkFyaWFsIiBmb250LXNpemU9IjEyIiBmaWxsPSIjOTk5IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBkeT0iLjNlbSI+5Zu+54mHPC90ZXh0Pjwvc3ZnPg==";
  }
}

// 直接打开网页
function openUrlDrawer(url: string) {
  // 显示确认对话框
  ElMessageBox.confirm(
    '您即将跳转到外部网站，是否继续？',
    '跳转确认',
    {
      confirmButtonText: '确认跳转',
      cancelButtonText: '取消',
      type: 'info',
      draggable: true,
      customStyle: {
        maxWidth: '400px'
      }
    }
  ).then(() => {
    // 用户确认后打开链接
    window.open(url, '_blank');
  }).catch(() => {
    // 用户取消，不做任何操作
    console.log('用户取消了跳转');
  });
}

// 监听数据源变化，滚动到顶部
import { watch, nextTick } from 'vue'
watch(() => props.activeSource, async () => {
  await nextTick();
  if (contentBodyRef.value) {
    contentBodyRef.value.scrollTop = 0;
  }
});

// 监听loading状态变化，当数据加载完成时也滚动到顶部
watch(() => props.loading, async (newLoading, oldLoading) => {
  if (oldLoading && !newLoading && !props.error) {
    await nextTick();
    if (contentBodyRef.value) {
      contentBodyRef.value.scrollTop = 0;
    }
  }
});
</script>

<style scoped>
.news-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: white;
}

.content-header {
  padding: 1.5rem;
  border-bottom: 1px solid #e4e7ed;
  background: white;
}

.header-info h2 {
  margin: 0 0 0.5rem 0;
  color: #303133;
  font-size: 1.5rem;
  font-weight: 600;
}

.header-info p {
  margin: 0 0 1rem 0;
  color: #606266;
  font-size: 0.9rem;
}

.header-meta {
  display: flex;
  gap: 1rem;
  font-size: 0.8rem;
  color: #909399;
}

.content-body {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
}

.loading-container,
.error-container,
.empty-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 200px;
  color: #909399;
}

.loading-icon,
.error-icon,
.empty-icon {
  font-size: 2rem;
  margin-bottom: 1rem;
}

.news-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.news-item {
  display: flex;
  padding: 1rem;
  border: 1px solid #e4e7ed;
  border-radius: 8px;
  background: white;
  cursor: pointer;
  transition: all 0.3s ease;
}

.news-item:hover {
  border-color: #409eff;
  box-shadow: 0 2px 12px rgba(64, 158, 255, 0.1);
}

.news-cover {
  width: 80px;
  height: 60px;
  border-radius: 6px;
  overflow: hidden;
  flex-shrink: 0;
  margin-right: 1rem;
}

.news-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transition: transform 0.3s ease;
}

.news-item:hover .news-cover img {
  transform: scale(1.05);
}

.news-main {
  flex: 1;
  min-width: 0;
}

.news-title {
  margin: 0 0 0.5rem 0;
  font-size: 1rem;
  font-weight: 500;
  color: #303133;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.news-desc {
  margin: 0 0 0.5rem 0;
  color: #606266;
  font-size: 0.85rem;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.news-meta {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
  color: #909399;
}

.news-author {
  color: #409eff;
}

.news-hot,
.news-time {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.news-hot {
  color: #f56c6c;
  font-weight: 500;
}

.news-action {
  display: flex;
  align-items: center;
  color: #c0c4cc;
  transition: color 0.3s ease;
  cursor: pointer;
}

.news-action:hover {
  color: #409eff;
}

@media (max-width: 768px) {
  .content-header {
    padding: 1rem;
  }
  
  .content-body {
    padding: 0.5rem;
  }
  
  .news-item {
    padding: 0.75rem;
  }
  
  .news-cover {
    width: 60px;
    height: 45px;
    margin-right: 0.75rem;
  }
  
  .news-title {
    font-size: 0.9rem;
  }
  
  .news-desc {
    font-size: 0.8rem;
  }
}
</style>
