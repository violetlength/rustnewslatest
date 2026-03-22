<script setup lang="ts">
import { Refresh, Delete, Menu } from "@element-plus/icons-vue";

interface Props {
  activeSource: string;
  sidebarCollapsed?: boolean;
}

interface Emits {
  (e: "refresh"): void;
  (e: "clear-cache"): void;
  (e: "toggle-sidebar"): void;
}

withDefaults(defineProps<Props>(), {
  sidebarCollapsed: false
});

defineEmits<Emits>();
</script>

<template>
  <header class="news-header">
    <div class="header-content">
      <div class="header-left">
        <el-button 
          class="menu-toggle"
          :icon="Menu" 
          @click="$emit('toggle-sidebar')"
          :title="sidebarCollapsed ? '展开侧边栏' : '折叠侧边栏'"
          circle
          size="small"
        />
        <div class="logo">
          <h1>NewsLatest</h1>
          <span class="subtitle">今日热榜</span>
        </div>
      </div>
      <div class="actions">
        <el-button 
          type="primary" 
          :icon="Refresh" 
          @click="$emit('refresh')"
          title="刷新当前数据源"
        >
          刷新当前数据
        </el-button>
        <el-button 
          type="danger" 
          :icon="Delete" 
          @click="$emit('clear-cache')"
          title="清空所有缓存"
        >
          清空所有缓存
        </el-button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.news-header {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  padding: 0;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 2rem;
  max-width: 100%;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.menu-toggle {
  background: rgba(255, 255, 255, 0.2);
  border: none;
  color: white;
  backdrop-filter: blur(10px);
  transition: all 0.3s ease;
}

.menu-toggle:hover {
  background: rgba(255, 255, 255, 0.3);
  transform: translateY(-1px);
}

.logo {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
}

.logo h1 {
  font-size: 1.5rem;
  font-weight: 600;
  margin: 0;
  color: white;
}

.subtitle {
  font-size: 0.9rem;
  opacity: 0.8;
  font-weight: 300;
}

.actions {
  display: flex;
  gap: 0.5rem;
}

.actions .el-button {
  border: none;
  background: rgba(255, 255, 255, 0.2);
  color: white;
  backdrop-filter: blur(10px);
  transition: all 0.3s ease;
}

.actions .el-button:hover {
  background: rgba(255, 255, 255, 0.3);
  transform: translateY(-1px);
}

.actions .el-button--danger {
  background: rgba(245, 108, 108, 0.8);
}

.actions .el-button--danger:hover {
  background: rgba(245, 108, 108, 0.9);
}

@media (max-width: 768px) {
  .header-content {
    padding: 1rem;
    flex-direction: column;
    gap: 1rem;
    align-items: stretch;
  }
  
  .header-left {
    justify-content: space-between;
    width: 100%;
  }
  
  .logo {
    justify-content: center;
  }
  
  .actions {
    justify-content: center;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  
  .actions .el-button {
    font-size: 0.85rem;
    padding: 0.5rem 1rem;
  }
}
</style>
