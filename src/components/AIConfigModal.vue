<template>
  <div class="modal-overlay" @click="closeModal">
    <div class="modal-content" @click.stop>
      <div class="modal-header">
        <h2>AI configuration</h2>
        <button class="close-btn" @click="closeModal">×</button>
      </div>
      
      <div class="modal-body">
        <div class="form-group">
          <label for="provider">AI provider</label>
          <select id="provider" v-model="config.provider" @change="updateModelOptions">
            <option value="">Select a provider</option>
            <optgroup label="International">
              <option value="openai">OpenAI</option>
              <option value="anthropic">Anthropic Claude</option>
              <option value="azure">Azure OpenAI</option>
            </optgroup>
            <optgroup label="Chinese AI Models">
              <option value="deepseek">DeepSeek</option>
              <option value="moonshot">Moonshot AI (Kimi)</option>
              <option value="zhipuai">Zhipu AI (GLM)</option>
              <option value="qwen">Alibaba Qwen</option>
              <option value="baichuan">Baichuan AI</option>
              <option value="doubao">Volcengine Doubao</option>
              <option value="baidu">Baidu ERNIE</option>
            </optgroup>
          </select>
          <div v-if="config.provider" class="provider-info">
            <small class="help-text">
              <strong>{{ getProviderName(config.provider) }}</strong>: {{ getProviderDescription(config.provider) }}
            </small>
          </div>
        </div>
        
        <div class="form-group">
          <label for="api_key">API Key</label>
          <input 
            id="api_key"
            v-model="config.api_key"
            type="password"
            placeholder="Enter your API key"
            autocomplete="off"
          />
        </div>
        
        <div class="form-group" v-if="config.provider && ['azure', 'deepseek', 'moonshot', 'zhipuai', 'qwen', 'baichuan', 'doubao', 'baidu'].includes(config.provider)">
          <label for="api_base">API Base URL</label>
          <input 
            id="api_base"
            v-model="config.api_base"
            type="text"
            :placeholder="getApiBasePlaceholder()"
          />
          <small class="help-text" v-if="config.provider">
            {{ getApiBaseHelp() }}
          </small>
        </div>
        
        <div class="form-group">
          <label for="model">Model</label>
          <select id="model" v-model="config.model">
            <option value="">Select a model</option>
            <option v-for="model in availableModels" :key="model" :value="model">
              {{ model }}
            </option>
          </select>
        </div>
        
        <div class="form-group">
          <label class="checkbox-label">
            <input 
              type="checkbox"
              v-model="config.enabled"
            />
            Enable AI parsing
          </label>
          <small class="help-text">
            When enabled, AI will be used to parse web content for better accuracy
          </small>
        </div>
        
        <div class="test-section" v-if="config.api_key && config.model">
          <button 
            class="test-btn"
            @click="testConnection"
            :disabled="testing"
          >
            {{ testing ? 'Testing...' : 'Test Connection' }}
          </button>
          <div v-if="testResult" class="test-result" :class="{ success: testResult.success, error: !testResult.success }">
            {{ testResult.message }}
          </div>
        </div>
      </div>
      
      <div class="modal-footer">
        <button class="btn-cancel" @click="closeModal">Cancel</button>
        <button class="btn-save" @click="saveConfig" :disabled="!isValid">
          Save Configuration
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useApi } from '@/composables/useApi'

const emit = defineEmits(['close', 'saved'])
const api = useApi()

const config = ref({
  provider: '',
  api_key: '',
  model: '',
  api_base: '',
  enabled: false
})

const testing = ref(false)
const testResult = ref(null)

const providerModels = {
  // International models
  openai: ['gpt-3.5-turbo', 'gpt-4', 'gpt-4-turbo'],
  anthropic: ['claude-3-haiku-20240307', 'claude-3-sonnet-20240229', 'claude-3-opus-20240229'],
  azure: ['gpt-35-turbo', 'gpt-4', 'gpt-4-32k'],
  
  // Chinese AI models
  deepseek: ['deepseek-chat', 'deepseek-coder'],
  moonshot: ['moonshot-v1-8k', 'moonshot-v1-32k', 'moonshot-v1-128k'],
  zhipuai: ['glm-4', 'glm-4-0520', 'glm-4-air', 'glm-4-flash', 'glm-3-turbo'],
  qwen: ['qwen-turbo', 'qwen-plus', 'qwen-max', 'qwen2-72b-instruct', 'qwen2-57b-llm'],
  baichuan: ['Baichuan2-Turbo', 'Baichuan2-Turbo-192k', 'Baichuan-Text-Embedding'],
  doubao: ['doubao-lite-4k', 'doubao-lite-32k', 'doubao-lite-128k', 'doubao-pro-4k', 'doubao-pro-32k', 'doubao-pro-128k'],
  baidu: ['ernie-3.5-8k', 'ernie-4.0-8k', 'ernie-turbo-8k', 'ernie-speed-8k', 'ernie-lite-8k']
}

const availableModels = computed(() => {
  return providerModels[config.value.provider] || []
})

const isValid = computed(() => {
  return config.value.provider && 
         config.value.api_key && 
         config.value.model
})

const updateModelOptions = () => {
  config.value.model = ''
  
  // Set default API base URLs for different providers
  const apiBaseUrls = {
    'azure': 'https://your-resource.openai.azure.com',
    'deepseek': 'https://api.deepseek.com/v1',
    'moonshot': 'https://api.moonshot.cn/v1',
    'zhipuai': 'https://open.bigmodel.cn/api/paas/v4',
    'qwen': 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    'baichuan': 'https://api.baichuan-ai.com/v1',
    'doubao': 'https://ark.cn-beijing.volces.com/api/v3',
    'baidu': 'https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop'
  }
  
  config.value.api_base = apiBaseUrls[config.value.provider] || ''
}

const loadConfig = async () => {
  try {
    const response = await api.get('/api/ai-config')
    if (response.success) {
      config.value = { ...config.value, ...response.data.current_config }
    }
  } catch (error) {
    console.error('Failed to load AI config:', error)
  }
}

const saveConfig = async () => {
  try {
    const response = await api.post('/api/ai-config', config.value)
    if (response.success) {
      emit('saved', response.data)
      closeModal()
    }
  } catch (error) {
    console.error('Failed to save AI config:', error)
    alert('Failed to save configuration: ' + error.message)
  }
}

const testConnection = async () => {
  testing.value = true
  testResult.value = null
  
  try {
    // Save config temporarily for testing
    const testConfig = { ...config.value }
    await api.post('/api/ai-config', testConfig)
    
    // Test actual AI connection
    const testResponse = await api.post('/api/ai-test')
    
    if (testResponse.success) {
      testResult.value = {
        success: true,
        message: `AI connection successful! Provider: ${testResponse.data.provider}, Model: ${testResponse.data.model}. Response: ${testResponse.data.response}`
      }
    } else {
      testResult.value = {
        success: false,
        message: 'AI connection test failed: ' + (testResponse.message || testResponse.error || 'Unknown error')
      }
    }
  } catch (error) {
    testResult.value = {
      success: false,
      message: 'Connection failed: ' + error.message
    }
  } finally {
    testing.value = false
  }
}

const getApiBasePlaceholder = () => {
  const placeholders = {
    'azure': 'https://your-resource.openai.azure.com',
    'deepseek': 'https://api.deepseek.com/v1',
    'moonshot': 'https://api.moonshot.cn/v1',
    'zhipuai': 'https://open.bigmodel.cn/api/paas/v4',
    'qwen': 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    'baichuan': 'https://api.baichuan-ai.com/v1',
    'doubao': 'https://ark.cn-beijing.volces.com/api/v3',
    'baidu': 'https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop'
  }
  return placeholders[config.value.provider] || 'Enter API base URL'
}

const getApiBaseHelp = () => {
  const helpText = {
    'azure': 'Azure OpenAI endpoint URL',
    'deepseek': 'DeepSeek API endpoint',
    'moonshot': 'Moonshot AI API endpoint',
    'zhipuai': 'Zhipu AI API endpoint',
    'qwen': 'Qwen DashScope API endpoint',
    'baichuan': 'Baichuan AI API endpoint',
    'doubao': 'Volcengine Doubao API endpoint',
    'baidu': 'Baidu ERNIE API endpoint'
  }
  return helpText[config.value.provider] || ''
}

const getProviderName = (provider) => {
  const names = {
    'openai': 'OpenAI',
    'anthropic': 'Anthropic Claude',
    'azure': 'Azure OpenAI',
    'deepseek': 'DeepSeek',
    'moonshot': 'Moonshot AI (Kimi)',
    'zhipuai': 'Zhipu AI (GLM)',
    'qwen': 'Alibaba Qwen',
    'baichuan': 'Baichuan AI',
    'doubao': 'Volcengine Doubao',
    'baidu': 'Baidu ERNIE'
  }
  return names[provider] || provider
}

const getProviderDescription = (provider) => {
  const descriptions = {
    'openai': 'Powerful language models with excellent reasoning capabilities',
    'anthropic': 'Constitutional AI with strong safety features',
    'azure': 'Enterprise-grade OpenAI with Microsoft Azure integration',
    'deepseek': 'Advanced reasoning and code generation, cost-effective',
    'moonshot': 'Long context window (128K) and multilingual support',
    'zhipuai': 'GLM-4 series with strong Chinese language understanding',
    'qwen': 'Enterprise-ready models with robust performance',
    'baichuan': 'Long context (192K) and efficient processing',
    'doubao': 'Lightweight and cost-effective models',
    'baidu': 'Chinese-optimized models with strong cultural understanding'
  }
  return descriptions[provider] || 'AI language model provider'
}

const closeModal = () => {
  emit('close')
}

onMounted(() => {
  loadConfig()
})
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: white;
  border-radius: 8px;
  width: 90%;
  max-width: 500px;
  max-height: 80vh;
  overflow-y: auto;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px;
  border-bottom: 1px solid #e5e7eb;
}

.modal-header h2 {
  margin: 0;
  font-size: 1.5rem;
  color: #1f2937;
}

.close-btn {
  background: none;
  border: none;
  font-size: 1.5rem;
  cursor: pointer;
  color: #6b7280;
  padding: 0;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.close-btn:hover {
  color: #374151;
}

.modal-body {
  padding: 20px;
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  margin-bottom: 5px;
  font-weight: 500;
  color: #374151;
}

.form-group input,
.form-group select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 14px;
}

.form-group input:focus,
.form-group select:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
}

.form-group select optgroup {
  font-weight: 600;
  color: #374151;
  background-color: #f9fafb;
}

.form-group select optgroup option {
  font-weight: 400;
  color: #1f2937;
  background-color: white;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.checkbox-label input[type="checkbox"] {
  width: auto;
}

.help-text {
  display: block;
  margin-top: 5px;
  color: #6b7280;
  font-size: 12px;
}

.provider-info {
  margin-top: 8px;
  padding: 8px 12px;
  background-color: #f0f9ff;
  border-left: 3px solid #3b82f6;
  border-radius: 4px;
}

.provider-info .help-text {
  margin: 0;
  color: #1e40af;
  font-size: 13px;
  line-height: 1.4;
}

.test-section {
  margin-top: 20px;
  padding-top: 20px;
  border-top: 1px solid #e5e7eb;
}

.test-btn {
  background: #10b981;
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.test-btn:hover:not(:disabled) {
  background: #059669;
}

.test-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.test-result {
  margin-top: 10px;
  padding: 8px 12px;
  border-radius: 4px;
  font-size: 14px;
}

.test-result.success {
  background: #d1fae5;
  color: #065f46;
  border: 1px solid #a7f3d0;
}

.test-result.error {
  background: #fee2e2;
  color: #991b1b;
  border: 1px solid #fca5a5;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 20px;
  border-top: 1px solid #e5e7eb;
}

.btn-cancel {
  background: #6b7280;
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.btn-cancel:hover {
  background: #4b5563;
}

.btn-save {
  background: #3b82f6;
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.btn-save:hover:not(:disabled) {
  background: #2563eb;
}

.btn-save:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
