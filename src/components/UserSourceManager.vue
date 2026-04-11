<template>
  <div class="user-source-manager">
    <!-- 头部操作栏 -->
    <div class="manager-header">
      <h3>数据源管理</h3>
      <div class="header-actions">
        <el-button 
          type="success" 
          @click="showAIConfig = true"
          :icon="Setting"
        >
          AI配置
        </el-button>
        <el-button 
          type="primary" 
          @click="showCreateDialog = true"
          :icon="Plus"
        >
          添加数据源
        </el-button>
      </div>
    </div>

    <!-- 数据源列表 -->
    <div class="source-list">
      <el-empty 
        v-if="userSources.length === 0 && !loading"
        description="暂无自定义数据源"
      />
      
      <el-card 
        v-for="source in userSources" 
        :key="source.id"
        class="source-card"
        shadow="hover"
      >
        <template #header>
          <div class="card-header">
            <div class="source-info">
              <h4>{{ source.title }}</h4>
              <el-tag 
                :type="source.source_type === 'json' ? 'success' : 'warning'"
                size="small"
              >
                {{ source.source_type === 'json' ? 'JSON' : '网页' }}
              </el-tag>
            </div>
            <div class="source-actions">
              <el-button 
                type="danger" 
                size="small"
                @click="confirmDelete(source)"
                :icon="Delete"
                text
              >
                删除
              </el-button>
            </div>
          </div>
        </template>
        
        <div class="source-content">
          <p class="description">{{ source.description }}</p>
          <div class="source-details">
            <div class="detail-item">
              <span class="label">名称:</span>
              <span class="value">{{ source.name }}</span>
            </div>
            <div class="detail-item">
              <span class="label">URL:</span>
              <span class="value url">{{ source.url }}</span>
            </div>
            <div v-if="source.selector" class="detail-item">
              <span class="label">选择器:</span>
              <span class="value selector">{{ source.selector }}</span>
            </div>
            <div class="detail-item">
              <span class="label">创建时间:</span>
              <span class="value">{{ formatDate(source.created_at) }}</span>
            </div>
          </div>
        </div>
      </el-card>
    </div>

    <!-- 创建数据源对话框 -->
    <el-dialog
      v-model="showCreateDialog"
      title="添加自定义数据源"
      width="500px"
      :before-close="handleDialogClose"
    >
      <el-form
        ref="formRef"
        :model="createForm"
        :rules="formRules"
        label-width="80px"
      >
        <el-form-item label="名称" prop="name">
          <el-input
            v-model="createForm.name"
            placeholder="请输入数据源名称（英文，用于标识）"
          />
        </el-form-item>
        
        <el-form-item label="标题" prop="title">
          <el-input
            v-model="createForm.title"
            placeholder="请输入数据源标题"
          />
        </el-form-item>
        
        <el-form-item label="描述" prop="description">
          <el-input
            v-model="createForm.description"
            type="textarea"
            :rows="3"
            placeholder="请输入数据源描述"
          />
        </el-form-item>
        
        <el-form-item label="类型" prop="source_type">
          <el-radio-group v-model="createForm.source_type">
            <el-radio value="json">JSON API</el-radio>
            <el-radio value="web">网页抓取</el-radio>
          </el-radio-group>
        </el-form-item>
        
        <el-form-item label="URL" prop="url">
          <el-input
            v-model="createForm.url"
            placeholder="请输入数据源URL"
          />
        </el-form-item>
        
        <el-form-item 
          v-if="createForm.source_type === 'web'"
          label="CSS选择器" 
          prop="selector"
        >
          <el-input
            v-model="createForm.selector"
            placeholder="请输入CSS选择器，如: .news-item"
          />
          <div class="form-tip">
            <el-text size="small" type="info">
              用于从网页中提取新闻元素的CSS选择器
            </el-text>
          </div>
        </el-form-item>
      </el-form>
      
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="handleDialogClose">取消</el-button>
          <el-button 
            type="primary" 
            @click="handleCreateSource"
            :loading="creating"
          >
            {{ creating ? '创建中...' : '创建' }}
          </el-button>
        </div>
      </template>
    </el-dialog>
    
    <!-- AI configuration modal -->
    <AIConfigModal 
      v-if="showAIConfig"
      @close="showAIConfig = false"
      @saved="handleAIConfigSaved"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Delete, Setting } from '@element-plus/icons-vue'
import UserSourceService, { type UserNewsSource, type CreateUserSourceRequest } from '../services/userSource'
import AIConfigModal from './AIConfigModal.vue'

// 响应式数据
const userSources = ref<UserNewsSource[]>([])
const loading = ref(false)
const showCreateDialog = ref(false)
const showAIConfig = ref(false)
const creating = ref(false)
const formRef = ref<FormInstance>()

// 创建表单数据
const createForm = ref<CreateUserSourceRequest>({
  name: '',
  title: '',
  description: '',
  source_type: 'json',
  url: '',
  selector: ''
})

// 表单验证规则
const formRules: FormRules = {
  name: [
    { required: true, message: '请输入数据源名称', trigger: 'blur' },
    { pattern: /^[a-zA-Z0-9_-]+$/, message: '名称只能包含字母、数字、下划线和连字符', trigger: 'blur' }
  ],
  title: [
    { required: true, message: '请输入数据源标题', trigger: 'blur' }
  ],
  description: [
    { required: true, message: '请输入数据源描述', trigger: 'blur' }
  ],
  source_type: [
    { required: true, message: '请选择数据源类型', trigger: 'change' }
  ],
  url: [
    { required: true, message: '请输入数据源URL', trigger: 'blur' },
    { 
      validator: (_rule, value, callback) => {
        try {
          new URL(value)
          callback()
        } catch {
          callback(new Error('请输入有效的URL格式'))
        }
      }, 
      trigger: 'blur' 
    }
  ],
  selector: [
    { 
      validator: (_rule, value, callback) => {
        if (createForm.value.source_type === 'web' && !value) {
          callback(new Error('网页类型数据源必须提供CSS选择器'))
        } else {
          callback()
        }
      }, 
      trigger: 'blur' 
    }
  ]
}

// 获取用户数据源列表
const loadUserSources = async () => {
  try {
    loading.value = true
    userSources.value = await UserSourceService.getUserSources()
  } catch (error) {
    console.error('加载用户数据源失败:', error)
    ElMessage.error('加载用户数据源失败')
  } finally {
    loading.value = false
  }
}

// 创建数据源
const handleCreateSource = async () => {
  if (!formRef.value) return
  
  try {
    await formRef.value.validate()
    creating.value = true
    
    const newSource = await UserSourceService.createUserSource(createForm.value)
    userSources.value.push(newSource)
    
    ElMessage.success('数据源创建成功')
    showCreateDialog.value = false
    resetForm()
  } catch (error) {
    console.error('创建数据源失败:', error)
    ElMessage.error(error instanceof Error ? error.message : '创建数据源失败')
  } finally {
    creating.value = false
  }
}

// 确认删除数据源
const confirmDelete = (source: UserNewsSource) => {
  ElMessageBox.confirm(
    `确定要删除数据源 "${source.title}" 吗？此操作不可恢复。`,
    '确认删除',
    {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning',
    }
  ).then(async () => {
    try {
      await UserSourceService.deleteUserSource(source.id)
      const index = userSources.value.findIndex(s => s.id === source.id)
      if (index > -1) {
        userSources.value.splice(index, 1)
      }
      ElMessage.success('数据源删除成功')
    } catch (error) {
      console.error('删除数据源失败:', error)
      ElMessage.error(error instanceof Error ? error.message : '删除数据源失败')
    }
  }).catch(() => {
    // 用户取消删除
  })
}

// 重置表单
const resetForm = () => {
  createForm.value = {
    name: '',
    title: '',
    description: '',
    source_type: 'json',
    url: '',
    selector: ''
  }
  formRef.value?.clearValidate()
}

// 关闭对话框
const handleDialogClose = () => {
  showCreateDialog.value = false
  resetForm()
}

// 格式化日期
const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString('zh-CN')
}

// Handle AI configuration saved
const handleAIConfigSaved = (config: any) => {
  ElMessage.success('AI configuration saved successfully')
  showAIConfig.value = false
}

// 组件挂载时加载数据
onMounted(() => {
  loadUserSources()
})
</script>

<style scoped>
.user-source-manager {
  padding: 20px;
}

.manager-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.manager-header h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.source-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
  gap: 20px;
}

.source-card {
  height: fit-content;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.source-info h4 {
  margin: 0 0 8px 0;
  font-size: 16px;
  font-weight: 600;
}

.source-actions {
  display: flex;
  gap: 8px;
}

.source-content {
  padding: 0;
}

.description {
  margin: 0 0 16px 0;
  color: #666;
  line-height: 1.5;
}

.source-details {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.detail-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

.label {
  font-weight: 500;
  color: #333;
  min-width: 80px;
  flex-shrink: 0;
}

.value {
  color: #666;
  word-break: break-all;
  flex: 1;
}

.value.url {
  font-family: monospace;
  font-size: 12px;
  background: #f5f5f5;
  padding: 2px 4px;
  border-radius: 3px;
}

.value.selector {
  font-family: monospace;
  font-size: 12px;
  background: #fff3cd;
  padding: 2px 4px;
  border-radius: 3px;
}

.form-tip {
  margin-top: 4px;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

/* 移动端适配 */
@media (max-width: 768px) {
  .user-source-manager {
    padding: 16px;
  }
  
  .manager-header {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
  }
  
  .source-list {
    grid-template-columns: 1fr;
    gap: 16px;
  }
  
  .card-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
  
  .source-info {
    width: 100%;
  }
  
  .source-actions {
    width: 100%;
    justify-content: flex-end;
  }
}
</style>
