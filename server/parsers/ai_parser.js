const fs = require('fs').promises;
const path = require('path');
const axios = require('axios');

class AIParser {
  constructor() {
    this.configPath = path.join(__dirname, '../config/ai_config.json');
    this.config = null;
  }

  async loadConfig() {
    try {
      const configData = await fs.readFile(this.configPath, 'utf8');
      this.config = JSON.parse(configData);
    } catch (error) {
      console.error('Failed to load AI config:', error.message);
      this.config = {
        current_config: {
          provider: "",
          api_key: "",
          model: "",
          api_base: "",
          enabled: false
        }
      };
    }
  }

  async saveConfig(config) {
    try {
      await fs.writeFile(this.configPath, JSON.stringify(config, null, 2));
      this.config = config;
      return true;
    } catch (error) {
      console.error('Failed to save AI config:', error.message);
      return false;
    }
  }

  async getConfig() {
    if (!this.config) {
      await this.loadConfig();
    }
    return this.config;
  }

  async updateConfig(newConfig) {
    const currentConfig = await this.getConfig();
    currentConfig.current_config = { ...currentConfig.current_config, ...newConfig };
    await this.saveConfig(currentConfig);
    return currentConfig.current_config;
  }

  async parseWebContent(url, htmlContent, selector = '') {
    const config = await this.getConfig();
    const { provider, api_key, model, api_base, enabled } = config.current_config;

    if (!enabled || !api_key) {
      throw new Error('AI parsing is not configured or enabled');
    }

    const prompt = this.buildPrompt(url, htmlContent, selector);
    
    try {
      let response;
      
      if (provider === 'openai' || provider === 'azure' || 
          provider === 'deepseek' || provider === 'moonshot' || 
          provider === 'qwen' || provider === 'baichuan' || 
          provider === 'doubao') {
        response = await this.callOpenAI(api_key, api_base, model, prompt);
      } else if (provider === 'anthropic') {
        response = await this.callAnthropic(api_key, model, prompt);
      } else if (provider === 'zhipuai') {
        response = await this.callZhipuAI(api_key, api_base, model, prompt);
      } else if (provider === 'baidu') {
        response = await this.callBaiduERNIE(api_key, api_base, model, prompt);
      } else {
        throw new Error(`Unsupported AI provider: ${provider}`);
      }

      return this.parseAIResponse(response);
    } catch (error) {
      console.error('AI parsing failed:', error.message);
      throw error;
    }
  }

  buildPrompt(url, htmlContent, selector) {
    const truncatedHtml = htmlContent.length > 50000 
      ? htmlContent.substring(0, 50000) + '...' 
      : htmlContent;

    return `You are a web content parser. Extract news items from the following HTML content and return them in a specific JSON format.

URL: ${url}
Selector: ${selector || 'None (auto-detect)'}

HTML Content:
${truncatedHtml}

Please extract news items and return ONLY a JSON array with the following structure:
[
  {
    "title": "News title",
    "url": "Full URL to the news article",
    "desc": "Brief description or summary",
    "timestamp": "ISO 8601 timestamp or date string",
    "author": "Author name if available",
    "cover": "Image URL if available",
    "hot": "Popularity score or views if available"
  }
]

Requirements:
1. Extract up to 20 most recent news items
2. Ensure all URLs are absolute (include full domain)
3. Filter out navigation, menu, and non-news content
4. If no selector is provided, use intelligent detection to find news patterns
5. Return ONLY the JSON array, no additional text
6. If no news items found, return an empty array []`;
  }

  async callOpenAI(apiKey, apiBase, model, prompt) {
    const response = await axios.post(
      `${apiBase}/chat/completions`,
      {
        model: model,
        messages: [
          {
            role: "user",
            content: prompt
          }
        ],
        temperature: 0.1,
        max_tokens: 4000
      },
      {
        headers: {
          'Authorization': `Bearer ${apiKey}`,
          'Content-Type': 'application/json'
        }
      }
    );

    return response.data.choices[0].message.content;
  }

  async callAnthropic(apiKey, model, prompt) {
    const response = await axios.post(
      'https://api.anthropic.com/v1/messages',
      {
        model: model,
        max_tokens: 4000,
        messages: [
          {
            role: "user",
            content: prompt
          }
        ]
      },
      {
        headers: {
          'x-api-key': apiKey,
          'Content-Type': 'application/json',
          'anthropic-version': '2023-06-01'
        }
      }
    );

    return response.data.content[0].text;
  }

  async callZhipuAI(apiKey, apiBase, model, prompt) {
    const response = await axios.post(
      `${apiBase}/chat/completions`,
      {
        model: model,
        messages: [
          {
            role: "user",
            content: prompt
          }
        ],
        max_tokens: 4000,
        temperature: 0.7
      },
      {
        headers: {
          'Authorization': `Bearer ${apiKey}`,
          'Content-Type': 'application/json'
        }
      }
    );

    return response.data.choices[0].message.content;
  }

  async callBaiduERNIE(apiKey, apiBase, model, prompt) {
    // Baidu ERNIE requires access token, using apiKey as access_token for simplicity
    const response = await axios.post(
      `${apiBase}/chat/${model}`,
      {
        messages: [
          {
            role: "user",
            content: prompt
          }
        ],
        temperature: 0.7,
        max_output_tokens: 4000
      },
      {
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${apiKey}`
        }
      }
    );

    return response.data.result;
  }

  parseAIResponse(response) {
    try {
      // Clean up the response to extract JSON
      let jsonStr = response.trim();
      
      // Remove any markdown code blocks
      jsonStr = jsonStr.replace(/^```json\n/, '').replace(/\n```$/, '');
      
      console.log('AI raw response:', jsonStr);
      
      // Try to parse as JSON
      const items = JSON.parse(jsonStr);
      
      console.log('AI parsed items:', items);
      
      // Validate and format items
      if (!Array.isArray(items)) {
        throw new Error('AI response is not an array');
      }

      return items.map((item, index) => ({
        id: require('uuid').v4(),
        title: item.title || `Item ${index + 1}`,
        url: item.url || new URL(item.link || '', url).href,
        desc: item.desc || '',
        cover: item.cover || null,
        author: item.author || null,
        timestamp: item.timestamp || new Date().toISOString(),
        hot: item.hot || null,
        mobile_url: null
      }));
    } catch (error) {
      console.error('Failed to parse AI response:', error.message);
      console.error('Raw response:', response);
      throw new Error('Invalid AI response format');
    }
  }

  async parseJsonResponse(url, jsonData) {
    const config = await this.getConfig();
    const { provider, api_key, model, api_base, enabled } = config.current_config;

    if (!enabled || !api_key) {
      throw new Error('AI parsing is not configured or enabled');
    }

    const prompt = this.buildJsonPrompt(url, jsonData);
    
    try {
      let response;
      
      if (provider === 'openai' || provider === 'azure' || 
          provider === 'deepseek' || provider === 'moonshot' || 
          provider === 'qwen' || provider === 'baichuan' || 
          provider === 'doubao') {
        response = await this.callOpenAI(api_key, api_base, model, prompt);
      } else if (provider === 'anthropic') {
        response = await this.callAnthropic(api_key, model, prompt);
      } else if (provider === 'zhipuai') {
        response = await this.callZhipuAI(api_key, api_base, model, prompt);
      } else if (provider === 'baidu') {
        response = await this.callBaiduERNIE(api_key, api_base, model, prompt);
      } else {
        throw new Error(`Unsupported AI provider: ${provider}`);
      }

      return this.parseAIResponse(response);
    } catch (error) {
      console.error('AI JSON parsing failed:', error.message);
      throw error;
    }
  }

  buildJsonPrompt(url, jsonData) {
    const jsonStr = typeof jsonData === 'string' 
      ? jsonData 
      : JSON.stringify(jsonData, null, 2);

    return `You are a data parser. Extract news items from the following JSON data and return them in a specific JSON format.

URL: ${url}

JSON Data:
${jsonStr}

Please extract news items and return ONLY a JSON array with the following structure:
[
  {
    "title": "News title",
    "url": "Full URL to the news article",
    "desc": "Brief description or summary",
    "timestamp": "ISO 8601 timestamp or date string",
    "author": "Author name if available",
    "cover": "Image URL if available",
    "hot": "Popularity score or views if available"
  }
]

Requirements:
1. Extract up to 20 most recent news items
2. Ensure all URLs are absolute (include full domain)
3. Filter out non-news content
4. Return ONLY the JSON array, no additional text
5. If no news items found, return an empty array []`;
  }
}

module.exports = new AIParser();
