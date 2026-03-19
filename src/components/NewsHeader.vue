<template>
  <header class="news-header">
    <div class="header-content">
      <h1 class="header-title">
        <el-icon><Document /></el-icon>
        RustNewsLatest
      </h1>
      <div class="header-actions">
        <el-button 
          type="primary" 
          :icon="Refresh" 
          :loading="loading"
          @click="$emit('refresh')"
        >
          刷新数据
        </el-button>
        <el-button 
          type="danger" 
          :icon="Delete"
          @click="$emit('clearCache')"
        >
          清空缓存
        </el-button>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { Document, Refresh, Delete } from '@element-plus/icons-vue'

interface Props {
  activeSource: string
  loading?: boolean
}

defineProps<Props>()

defineEmits<{
  refresh: []
  clearCache: []
}>()
</script>

<style scoped>
.news-header {
  background: white;
  border-bottom: 1px solid #e4e7ed;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.header-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 2rem;
  max-width: 1200px;
  margin: 0 auto;
}

.header-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
  color: #303133;
}

.header-title .el-icon {
  font-size: 1.8rem;
  color: #409eff;
}

.header-actions {
  display: flex;
  gap: 1rem;
}

@media (max-width: 768px) {
  .header-content {
    padding: 1rem;
    flex-direction: column;
    gap: 1rem;
  }
  
  .header-title {
    font-size: 1.2rem;
  }
  
  .header-actions {
    width: 100%;
    justify-content: center;
  }
}
</style>
