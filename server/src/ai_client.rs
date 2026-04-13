use super::ai_config::*;
use super::types::*;
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::{info, error};
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;

pub struct AIClient {
    client: Client,
    config: AIConfig,
}

impl AIClient {
    pub async fn new() -> Result<Self> {
        let config = AIConfig::load().await
            .map_err(|e| anyhow!("Failed to load AI config: {}", e))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("RustNewsLatest/1.0")
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(AIClient { client, config })
    }

    // ==================== HTML Mode ====================

    pub async fn parse_news_from_html(
        &self,
        url: &str,
        html_content: &str,
        selector: Option<&str>,
    ) -> Result<Vec<NewsItem>> {
        if !self.config.current_config.enabled {
            return Err(anyhow!("AI parsing is not enabled"));
        }

        let prompt = self.build_html_extraction_prompt(url, html_content, selector);

        let response = self.call_ai_provider(&prompt).await?;

        self.parse_html_ai_response(&response, url)
    }

    pub async fn generate_parsing_rules(
        &self,
        url: &str,
        html_content: &str,
    ) -> Result<ParsingRules> {
        if !self.config.current_config.enabled {
            return Err(anyhow!("AI parsing is not enabled"));
        }

        let prompt = self.build_legacy_rules_prompt(url, html_content);

        let response = self.call_ai_provider(&prompt).await?;

        self.parse_legacy_rules_response(&response, url)
    }

    pub async fn generate_structured_extraction_rules(
        &self,
        url: &str,
        html_content: &str,
    ) -> Result<ExtractionRules> {
        if !self.config.current_config.enabled {
            return Ok(ExtractionRules {
                task: "extract_news_list".to_string(),
                source_url: url.to_string(),
                rules_version: "1.0".to_string(),
                selectors: ExtractionSelectors {
                    root_container: "body".to_string(),
                    item_list: "article, .item, .news-item, li".to_string(),
                    item_node: "article, .item, .news-item, li".to_string(),
                    fields: std::collections::HashMap::new(),
                },
                notes: Some("Basic extraction rules generated (AI disabled)".to_string()),
                created_at: Utc::now(),
                success_rate: 0.8,
                total_attempts: 1,
            });
        }

        info!("Generating structured extraction rules for: {}", url);

        let prompt = self.build_structured_rules_prompt(url, html_content);

        info!("Calling AI provider: {} for structured extraction rules", self.config.current_config.provider);
        let response = self.call_ai_provider(&prompt).await?;

        self.parse_structured_rules_response(&response, url)
    }

    fn build_html_extraction_prompt(&self, url: &str, html_content: &str, selector: Option<&str>) -> String {
        let truncated_html = if html_content.len() > 50000 {
            let safe_end = (50000 - 10).max(0);
            let mut end = safe_end;
            while end > 0 && !html_content.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &html_content[..end])
        } else {
            html_content.to_string()
        };

        let selector_instruction = match selector {
            Some(sel) => format!("You MUST use the provided CSS selector '{}' to locate the items. Do not deviate.", sel),
            None => "You must intelligently detect the repeating pattern (e.g., <li>, <article>, or <tr>) to locate news items.".to_string()
        };

        format!(
            r#"You are an expert Data Extraction Engine. Your task is to parse the provided HTML and return a clean JSON dataset.

URL: {}
Instruction: {}

HTML Content:
{}

### Extraction Rules
1.  **Pattern Matching**: Identify the repeating items based on the instruction.
    -   **Handle Interleaved Rows**: If the data is in a table (like Hacker News) where the title is in one `<tr>` and details (score, comments) are in the *next* `<tr>`, you must combine them into a single object.
2.  **Field Extraction**: Extract **only** the fields that are actually present in the HTML.
    -   **title**: The main headline text.
    -   **url**: The link. **CRITICAL:** If the link is relative (starts with '/'), you must prepend the domain from the input URL (e.g., "https://news.dahe.cn").
    -   **timestamp**: Look for dates (YYYY-MM-DD) or relative time (e.g., "2 hours ago").
    -   **meta_info**: If there are scores, views, authors, or comments, extract them here. If none, leave empty.
    -   **description**: A brief summary if available.
3.  **Data Cleaning**:
    -   Remove extra whitespace and newlines.
    -   Filter out navigation links, "Read More" buttons, and ads.
4.  **Output Limit**: Extract the top 15-20 items.

### Output Format
Return **ONLY** a raw JSON array. Do not include markdown formatting (```json) or explanations.

Example Structure:
[
{{
    "title": "Example News Title",
    "url": "https://full-absolute-url.com/path",
    "timestamp": "2026-04-10 10:00",
    "meta_info": "100 points | by user123",
    "description": "Summary text..."
}}
]

If no valid news items are found, return an empty array: []
"#,
            url,
            selector_instruction,
            truncated_html
        )
    }

    fn build_legacy_rules_prompt(&self, url: &str, html_content: &str) -> String {
        let truncated_html = if html_content.len() > 50000 {
            let safe_end = (50000 - 10).max(0);
            let mut end = safe_end;
            while end > 0 && !html_content.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &html_content[..end])
        } else {
            html_content.to_string()
        };

        format!(
            r#"You are a Web Scraping Architect. Analyze the provided HTML and generate a universal extraction schema.

URL: {}
HTML Content:
{}

**Step 1: Structural Diagnosis**
Analyze the DOM tree to determine the layout pattern:
1. **Table/Interleaved Layout**: (Like Hacker News) Data is split across adjacent elements. *Strategy:* Use sibling selectors.
2. **List/Grid Layout**: (Like News Sites) Self-contained items (e.g., <li> or <div class="card">). *Strategy:* Find the repeating container.
3. **Shadow/Dynamic Layout**: (Like Modern JS Apps) Data might be obfuscated.

**Step 2: Rule Generation**
Generate a JSON object based on your diagnosis. Infer field names from the content (e.g., "views", "score", "author").

**Output JSON Schema:**
{{
"meta": {{
    "detected_pattern": "List" | "Table_Interleaved" | "Card_Grid",
    "confidence": "high" | "medium"
}},
"selectors": {{
    "root_container": "The outermost wrapper of the list (e.g., div#news, main)",
    "item_list": "The direct parent of repeating items (e.g., ul, ol, tbody)",
    "item_node": "The selector for one repeating item unit (e.g., li, .card, tr.athing)",
    "fields": {{
    "inferred_field_name_1": {{
        "selector": "CSS_SELECTOR_RELATIVE_TO_ITEM",
        "attribute": "text" | "href" | "src" | "datetime",
        "description": "What this field represents (e.g., Title, Publish Date)"
    }},
    "inferred_field_name_2": {{
        "selector": "CSS_SELECTOR_RELATIVE_TO_ITEM",
        "attribute": "text",
        "description": "Another field found in the HTML"
    }}
    // Add more fields as inferred from the HTML content
    }}
}},
"notes": "Any special instructions (e.g., 'Data is split into two rows')"
}}

**Constraints:**
- **Field Inference**: Do not limit yourself to 'title' and 'time'. Look at the HTML content. If you see numbers like '100 points', create a field like 'score'.
- **Robustness**: Avoid generic tags without classes unless necessary.
- **Output Format**: Return **ONLY** the raw JSON string. No markdown code blocks (```json), no explanations.
"#,
            url, truncated_html
        )
    }

    fn build_structured_rules_prompt(&self, url: &str, html_content: &str) -> String {
        let truncated_html = if html_content.len() > 50000 {
            let safe_end = (50000 - 10).max(0);
            let mut end = safe_end;
            while end > 0 && !html_content.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &html_content[..end])
        } else {
            html_content.to_string()
        };

        let content_type = self.detect_content_type_for_prompt(&truncated_html);

        let target_schema_example = r#"{
    "selectors": {
        "root_container": "div#main",
        "item_list": "ul.news-list",
        "item_node": "li.news-item",
        "fields": {
            "title": { "selector": "a", "attribute": "text" },
            "url": { "selector": "a", "attribute": "href" },
            "desc": { "selector": "p.summary", "attribute": "text" },
            "cover": { "selector": "img", "attribute": "src" },
            "author": { "selector": ".author", "attribute": "text" },
            "timestamp": { "selector": ".time", "attribute": "text" },
            "hot": { "selector": ".score", "attribute": "text" },
            "mobile_url": { "selector": "a", "attribute": "data-mobile" }
        }
    }
}"#;

        format!(
            r#"You are a Web Scraping Rule Generator. Your task is to generate precise CSS selectors for the provided HTML.

### TARGET DATA STRUCTURE
Your generated rules MUST map to these exact fields (to fit the Rust struct):
- title: (String, REQUIRED) The news headline.
- url: (String, REQUIRED) The link to the article. Can be relative (e.g., "/path", "//example.com/path") - system will auto-resolve.
- desc: (Option<String>) A short summary or description.
- cover: (Option<String>) Image URL. Can be relative - system will auto-resolve.
- author: (Option<String>) The writer or source name.
- timestamp: (Option<String>) Publish time.
- hot: (Option<String>) Popularity score (e.g., "100 points", "10k views").
- mobile_url: (Option<String>) Mobile redirect link. Can be relative - system will auto-resolve.

### INPUT
URL: {url}
Content Type: {content_type}
HTML: {truncated_html}

### INSTRUCTIONS
1. Analyze the HTML structure carefully.
2. Identify the hierarchical structure:
   - root_container: The outermost wrapper that contains the entire list section (e.g., div#main, table#hnmain)
   - item_list: The direct parent of the repeating items (e.g., ul.news-list, tbody)
   - item_node: The selector that matches EACH individual news item (e.g., li.news-item, tr.athing)
3. For each field in the TARGET DATA STRUCTURE above:
   - If the data exists in the HTML, provide a selector and attribute.
   - If the data does NOT exist, set that field to null.
4. Do NOT invent new field names. Only use the 8 fields listed above.

### URL HANDLING
The system automatically resolves relative URLs. Examples:
- "/article/123" → "https://example.com/article/123"
- "//cdn.example.com/img.jpg" → "https://cdn.example.com/img.jpg"
- "page.html" → "https://example.com/path/page.html"
You do NOT need to include base_url in your output. Just extract the raw href/src values.

### OUTPUT FORMAT
Return ONLY a JSON object in this exact format:
{target_schema_example}

### WARNINGS
- Do not include the raw HTML code inside the JSON values.
- Do not wrap the JSON in ```json code blocks.
- Do not add any text before or after the JSON.
- All selectors must be valid CSS selectors.
"#,
            url = url,
            content_type = content_type,
            truncated_html = truncated_html,
            target_schema_example = target_schema_example
        )
    }

    fn detect_content_type_for_prompt(&self, html_content: &str) -> String {
        let content_lower = html_content.to_lowercase();

        if content_lower.contains("<rss") || content_lower.contains("<feed") || content_lower.contains("<channel") {
            "RSS/Atom Feed".to_string()
        } else if content_lower.contains("<article") || content_lower.contains("class=\"article") || content_lower.contains("class=\"post\"") {
            "Article/Blog".to_string()
        } else if content_lower.contains("<ul") && content_lower.contains("<li>") && (content_lower.contains("href") || content_lower.contains("news")) {
            "List/News".to_string()
        } else if content_lower.contains("github.com") || content_lower.contains("repository") {
            "GitHub".to_string()
        } else if content_lower.contains("class=\"card\"") || content_lower.contains("class=\"item\"") {
            "Card-based Layout".to_string()
        } else {
            "General Web".to_string()
        }
    }

    fn parse_html_ai_response(&self, response: &str, base_url: &str) -> Result<Vec<NewsItem>> {
        let json_str = response.trim()
            .strip_prefix("```json")
            .unwrap_or(response)
            .strip_suffix("```")
            .unwrap_or(response)
            .trim();

        let raw_items: Vec<serde_json::Value> = serde_json::from_str(json_str)
            .map_err(|e| anyhow!("Failed to parse AI HTML response as JSON: {}", e))?;

        let mut news_items = Vec::new();
        for item in raw_items {
            if let Ok(news_item) = serde_json::from_value::<NewsItem>(item.clone()) {
                let mut final_item = news_item;
                if final_item.id.is_empty() {
                    final_item.id = uuid::Uuid::new_v4().to_string();
                }
                // 处理相对 URL
                final_item.url = self.make_absolute_url(&final_item.url, base_url);
                if let Some(cover) = &final_item.cover {
                    final_item.cover = Some(self.make_absolute_url(cover, base_url));
                }
                if let Some(mobile_url) = &final_item.mobile_url {
                    final_item.mobile_url = Some(self.make_absolute_url(mobile_url, base_url));
                }
                news_items.push(final_item);
            } else {
                let url = item.get("url").and_then(|v| v.as_str()).unwrap_or(base_url);
                let cover = item.get("cover").and_then(|v| v.as_str());
                let mobile_url = item.get("mobile_url").and_then(|v| v.as_str());
                
                news_items.push(NewsItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: item.get("title").and_then(|v| v.as_str()).unwrap_or("No Title").to_string(),
                    url: self.make_absolute_url(url, base_url),
                    desc: item.get("description")
                        .or_else(|| item.get("desc"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    cover: cover.map(|c| self.make_absolute_url(c, base_url)),
                    author: item.get("author")
                        .or_else(|| item.get("meta_info"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    timestamp: item.get("timestamp")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    hot: item.get("hot")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<u64>().ok())
                        .or_else(|| item.get("hot").and_then(|v| v.as_u64())),
                    mobile_url: mobile_url.map(|m| self.make_absolute_url(m, base_url)),
                });
            }
        }

        Ok(news_items)
    }

    fn make_absolute_url(&self, url: &str, base_url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else if url.starts_with("//") {
            // 协议相对 URL，如 //example.com/image.jpg
            if base_url.starts_with("https://") {
                format!("https:{}", url)
            } else {
                format!("http:{}", url)
            }
        } else if url.starts_with('/') {
            // 绝对路径，如 /image.jpg
            if let Ok(base) = url::Url::parse(base_url) {
                format!("{}://{}{}", base.scheme(), base.host_str().unwrap_or(""), url)
            } else {
                url.to_string()
            }
        } else {
            // 相对路径，如 image.jpg
            if let Some(last_slash) = base_url.rfind('/') {
                format!("{}{}", &base_url[..last_slash + 1], url)
            } else {
                format!("{}/{}", base_url, url)
            }
        }
    }

    fn parse_legacy_rules_response(&self, response: &str, _url: &str) -> Result<ParsingRules> {
        let cleaned_response = response.trim()
            .strip_prefix("```json")
            .unwrap_or(response)
            .strip_suffix("```")
            .unwrap_or(response)
            .trim();

        let rules_data: serde_json::Value = serde_json::from_str(cleaned_response)
            .map_err(|e| anyhow!("Failed to parse AI response as JSON: {}", e))?;

        let selectors = rules_data.get("selectors").unwrap_or(&rules_data);

        let container_selector = selectors.get("container_selector")
            .and_then(|v| v.as_str())
            .or_else(|| selectors.get("item_node").and_then(|v| v.as_str()))
            .ok_or_else(|| anyhow!("Missing container_selector in AI response"))?
            .to_string();

        let title_in_container = selectors.get("title_in_container")
            .and_then(|v| v.as_str())
            .or_else(|| {
                selectors.get("fields")
                    .and_then(|f| f.get("title"))
                    .and_then(|t| t.get("selector"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("a")
            .to_string();

        let link_in_container = selectors.get("link_in_container")
            .and_then(|v| v.as_str())
            .or_else(|| {
                selectors.get("fields")
                    .and_then(|f| f.get("url"))
                    .and_then(|t| t.get("selector"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("a")
            .to_string();

        let desc_in_container = selectors.get("desc_in_container")
            .and_then(|v| v.as_str())
            .or_else(|| {
                selectors.get("fields")
                    .and_then(|f| f.get("desc"))
                    .and_then(|t| t.get("selector"))
                    .and_then(|v| v.as_str())
            })
            .map(|s| s.to_string());

        let time_in_container = selectors.get("time_in_container")
            .and_then(|v| v.as_str())
            .or_else(|| {
                selectors.get("fields")
                    .and_then(|f| f.get("timestamp"))
                    .and_then(|t| t.get("selector"))
                    .and_then(|v| v.as_str())
            })
            .map(|s| s.to_string());

        Ok(ParsingRules {
            container_selector,
            title_in_container,
            link_in_container,
            desc_in_container,
            time_in_container,
            created_at: chrono::Utc::now(),
            success_rate: 1.0,
            total_attempts: 0,
        })
    }

    fn parse_structured_rules_response(&self, response: &str, url: &str) -> Result<ExtractionRules> {
        let cleaned_response = response
            .trim()
            .strip_prefix("```json")
            .unwrap_or(response)
            .strip_prefix("```")
            .unwrap_or(response)
            .strip_suffix("```")
            .unwrap_or(response)
            .trim();

        info!("Parsing AI rules response for URL: {}", url);

        #[derive(Deserialize)]
        struct TempField {
            selector: Option<String>,
            attribute: Option<String>,
        }

        #[derive(Deserialize)]
        struct TempSelectors {
            root_container: Option<String>,
            item_list: Option<String>,
            item_node: String,
            fields: std::collections::HashMap<String, Option<TempField>>,
        }

        #[derive(Deserialize)]
        struct TempResponse {
            selectors: TempSelectors,
        }

        let temp_data: TempResponse = serde_json::from_str(cleaned_response)
            .map_err(|e| {
                error!("Failed to parse AI JSON: {}. Raw response: {}", e, cleaned_response);
                anyhow!("Failed to parse AI JSON: {}. Raw: {}", e, cleaned_response)
            })?;

        info!("Successfully parsed AI response with item_node: {}", temp_data.selectors.item_node);

        let mut field_rules = std::collections::HashMap::new();

        for (field_name, field_data_opt) in temp_data.selectors.fields {
            let field_data = match field_data_opt {
                Some(data) => data,
                None => {
                    info!("Skipping field '{}' with null value", field_name);
                    continue;
                }
            };

            if let (Some(selector), Some(attribute)) = (field_data.selector, field_data.attribute) {
                let is_required = field_name == "title" || field_name == "url";

                let rule = FieldRule {
                    selector,
                    attribute,
                    required: is_required,
                    base_url: Some(url.to_string()),
                    format: None,
                    clean: true,
                };
                field_rules.insert(field_name, rule);
            } else {
                info!("Skipping field '{}' with null selector or attribute", field_name);
            }
        }

        let root_container = temp_data.selectors.root_container.unwrap_or_else(|| "body".to_string());
        let item_list = temp_data.selectors.item_list.unwrap_or_else(|| {
            let parts: Vec<&str> = temp_data.selectors.item_node.split(' ').collect();
            if parts.len() > 1 {
                parts[..parts.len()-1].join(" ")
            } else {
                "body".to_string()
            }
        });

        Ok(ExtractionRules {
            task: "extract_news_list".to_string(),
            source_url: url.to_string(),
            rules_version: "1.0".to_string(),
            selectors: ExtractionSelectors {
                root_container,
                item_list,
                item_node: temp_data.selectors.item_node,
                fields: field_rules,
            },
            notes: Some(format!("Auto-generated rules for {} on {}", url, chrono::Utc::now().format("%Y-%m-%d"))),
            created_at: chrono::Utc::now(),
            success_rate: 1.0,
            total_attempts: 0,
        })
    }

    // ==================== API Mode ====================

    pub async fn parse_news_from_json(
        &self,
        url: &str,
        json_content: &str,
        field_mapping_rules: Option<&serde_json::Value>,
    ) -> Result<Vec<NewsItem>> {
        if !self.config.current_config.enabled {
            return Err(anyhow!("AI parsing is not enabled"));
        }

        info!("Parsing JSON content from API: {}", url);

        if let Some(rules) = field_mapping_rules {
            info!("Using field mapping rules for JSON parsing");
            return self.apply_field_mapping_rules(json_content, rules, url).await;
        }

        info!("Raw JSON content to parse: {}", json_content);
        let json_value: serde_json::Value = serde_json::from_str(json_content)
            .map_err(|e| anyhow!("Failed to parse JSON content: {}", e))?;

        let news_items = self.parse_news_from_json_structure(&json_value, url).await?;

        info!("Successfully extracted {} news items from JSON", news_items.len());
        Ok(news_items)
    }

    pub async fn generate_field_mapping_rules(&self, json_data: &str, source_url: &str) -> Result<serde_json::Value> {
        if !self.config.current_config.enabled {
            return Err(anyhow!("AI parsing is not enabled"));
        }

        info!("Generating field mapping rules for JSON API: {}", source_url);

        let prompt = format!(
            r#"You are a JSON API data structure analyzer. Your task is to analyze the JSON data structure and generate field mapping rules for converting API responses to standard news item format.

TARGET NEWS ITEM SCHEMA:
- id: unique identifier (string, required)
- title: news title (string, required)
- desc: news description (string, optional)
- cover: news cover image URL (string, optional)
- author: author name (string, optional)
- timestamp: publication time (string, optional)
- hot: popularity score or view count (integer, optional)
- url: news article URL (string, required)
- mobile_url: mobile version URL (string, optional)

JSON API DATA:
{}

ANALYSIS INSTRUCTIONS:
1. Identify the array/object that contains the news items
2. Map each target field to the corresponding source field in the JSON
3. Choose appropriate transformation type based on data format
4. Set required fields correctly
5. Specify the data_path to access the news items array

TRANSFORM TYPES:
- direct: copy value directly (for exact matches)
- uuid: generate unique ID (ignores source field, for missing IDs)
- to_integer: convert string/number to integer (for counts, scores)
- prepend: add prefix to value (params: prefix string, for URLs)
- concat: append suffix to value (params: suffix string)
- truncate: truncate long text (params: max_length, for descriptions)
- format: format timestamp or other values
- null: always return null (for missing fields)

RESPONSE FORMAT (JSON only):
{{
  "field_mappings": {{
    "id": {{
      "source_field": "actual_field_name_in_json",
      "required": true,
      "transform": "transform_type",
      "transform_params": "parameters"
    }},
    "title": {{
      "source_field": "actual_field_name_in_json",
      "required": true,
      "transform": "transform_type",
      "transform_params": "parameters"
    }},
    "desc": {{
      "source_field": "actual_field_name_in_json",
      "required": false,
      "transform": "transform_type",
      "transform_params": "parameters"
    }},
    "cover": {{
      "source_field": "actual_field_name_in_json",
      "required": false,
      "transform": "transform_type",
      "transform_params": "parameters"
    }},
    "author": {{
      "source_field": "actual_field_name_in_json",
      "required": false,
      "transform": "transform_type",
      "transform_params": "parameters"
    }},
    "timestamp": {{
      "source_field": "actual_field_name_in_json",
      "required": false,
      "transform": "transform_type",
      "transform_params": "parameters"
    }},
    "hot": {{
      "source_field": "actual_field_name_in_json",
      "required": false,
      "transform": "transform_type",
      "transform_params": "parameters"
    }},
    "url": {{
      "source_field": "actual_field_name_in_json",
      "required": true,
      "transform": "transform_type",
      "transform_params": "parameters"
    }},
    "mobile_url": {{
      "source_field": "actual_field_name_in_json",
      "required": false,
      "transform": "transform_type",
      "transform_params": "parameters"
    }}
  }},
  "data_path": "path.to.news_items.array"
}}

IMPORTANT:
- Use actual field names from the JSON data
- Set data_path to the correct JSON path (e.g., "data", "items", "results", "articles")
- Only return valid JSON, no explanations
- Ensure all required fields (id, title, url) are mapped

Source API URL: {}"#,
            json_data, source_url
        );

        info!("Calling AI provider: {} for field mapping rules generation", self.config.current_config.provider);
        let response = self.call_ai_provider(&prompt).await?;

        info!("AI response length: {} chars", response.len());
        info!("AI response (first 1000 chars): {}", &response[..response.len().min(1000)]);

        let cleaned_response = response.trim()
            .strip_prefix("```json")
            .unwrap_or(&response)
            .strip_suffix("```")
            .unwrap_or(&response)
            .trim();

        let rules: serde_json::Value = serde_json::from_str(cleaned_response)
            .map_err(|e| anyhow!("Failed to parse AI field mapping rules: {}", e))?;

        info!("Generated field mapping rules: {}", serde_json::to_string_pretty(&rules)?);
        info!("Successfully generated field mapping rules for: {}", source_url);
        Ok(rules)
    }

    pub async fn apply_field_mapping_rules(&self, json_data: &str, rules: &serde_json::Value, source_url: &str) -> Result<Vec<NewsItem>> {
        info!("Applying field mapping rules, JSON data: {}", json_data);
        info!("Field mapping rules: {}", serde_json::to_string_pretty(rules)?);
        let json_value: serde_json::Value = serde_json::from_str(json_data)
            .map_err(|e| anyhow!("Failed to parse JSON data: {}", e))?;

        let field_mappings = rules.get("field_mappings")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow!("Invalid field mapping rules: missing field_mappings"))?;

        let data_path = rules.get("data_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let items_array = if data_path.is_empty() {
            json_value.as_array()
                .ok_or_else(|| anyhow!("JSON data is not an array and no data_path provided"))
                .map(|arr| arr.to_vec())
                .unwrap_or_default()
        } else {
            self.extract_nested_array(&json_value, data_path)?
        };

        let mut news_items = Vec::new();

        for (index, item) in items_array.iter().enumerate() {
            if let Some(news_item) = self.apply_mapping_to_item(item, field_mappings, index, source_url)? {
                news_items.push(news_item);
            }
        }

        info!("Applied field mapping rules, extracted {} news items", news_items.len());
        Ok(news_items)
    }

    fn extract_nested_array(&self, json_value: &serde_json::Value, path: &str) -> Result<Vec<serde_json::Value>> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json_value;

        for part in parts {
            current = current.get(part)
                .ok_or_else(|| anyhow!("Path '{}' not found in JSON data", part))?;
        }

        current.as_array()
            .ok_or_else(|| anyhow!("Path '{}' does not point to an array", path))
            .map(|arr| arr.to_vec())
    }

    fn apply_mapping_to_item(&self, item: &serde_json::Value, mappings: &serde_json::Map<String, serde_json::Value>, index: usize, source_url: &str) -> Result<Option<NewsItem>> {
        let mut news_item = NewsItem::default();

        for (target_field, mapping) in mappings {
            let source_field = mapping.get("source_field")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let transform = mapping.get("transform")
                .and_then(|v| v.as_str())
                .unwrap_or("direct");

            let transform_params = mapping.get("transform_params")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let required = mapping.get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let value = if source_field.is_empty() {
                None
            } else {
                item.get(source_field)
            };

            let processed_value = self.transform_field_value(value, transform, transform_params, index, source_url)?;
            let processed_value_clone = processed_value.clone();

            match target_field.as_str() {
                "id" => news_item.id = processed_value.unwrap_or_else(|| format!("item_{}", index)),
                "title" => news_item.title = processed_value.unwrap_or_else(|| "Untitled".to_string()),
                "desc" => news_item.desc = processed_value,
                "cover" => news_item.cover = processed_value,
                "author" => news_item.author = processed_value,
                "timestamp" => news_item.timestamp = processed_value,
                "hot" => {
                    if let Some(hot_str) = processed_value {
                        news_item.hot = hot_str.parse::<u64>().ok();
                    }
                },
                "url" => news_item.url = processed_value.unwrap_or_else(|| source_url.to_string()),
                "mobile_url" => news_item.mobile_url = processed_value,
                _ => {}
            }

            if required && processed_value_clone.is_none() {
                return Ok(None);
            }
        }

        if news_item.id.is_empty() || news_item.title.is_empty() || news_item.url.is_empty() {
            return Ok(None);
        }

        Ok(Some(news_item))
    }

    fn transform_field_value(&self, value: Option<&serde_json::Value>, transform: &str, params: &str, index: usize, _source_url: &str) -> Result<Option<String>> {
        match (value, transform) {
            (None, "null") => Ok(None),
            (None, "uuid") => Ok(Some(format!("item_{}", index))),
            (None, _) => Ok(None),
            (Some(v), "direct") => Ok(v.as_str().map(|s| s.to_string())),
            (Some(v), "to_integer") => {
                match v {
                    serde_json::Value::Number(n) => Ok(Some(n.to_string())),
                    serde_json::Value::String(s) => {
                        Ok(s.parse::<u64>()
                            .map(|n| Some(n.to_string()))
                            .unwrap_or(None))
                    }
                    _ => Ok(None)
                }
            },
            (Some(v), "prepend") => Ok(Some(format!("{}{}", params, v.as_str().unwrap_or("")))),
            (Some(v), "concat") => {
                if let Some(s) = v.as_str() {
                    Ok(Some(format!("{}{}", s, params)))
                } else {
                    Ok(None)
                }
            },
            (Some(v), "truncate") => {
                if let Some(s) = v.as_str() {
                    let max_len = params.parse::<usize>().unwrap_or(200);
                    Ok(Some(if s.len() > max_len {
                        let safe_len = if max_len >= 3 {
                            s.char_indices()
                                .take_while(|(byte_idx, _)| *byte_idx <= max_len.saturating_sub(3))
                                .last()
                                .map(|(byte_idx, _)| byte_idx)
                                .unwrap_or(0)
                        } else {
                            0
                        };
                        format!("{}...", &s[..safe_len])
                    } else {
                        s.to_string()
                    }))
                } else {
                    Ok(None)
                }
            },
            (Some(v), "format") => Ok(v.as_str().map(|s| s.to_string())),
            (Some(v), _) => Ok(v.as_str().map(|s| s.to_string())),
        }
    }

    async fn parse_news_from_json_structure(&self, json_value: &serde_json::Value, base_url: &str) -> Result<Vec<NewsItem>> {
        let mut news_items = Vec::new();

        if let Some(data) = json_value.get("data").and_then(|v| v.as_array()) {
            for (index, item) in data.iter().enumerate() {
                if let Some(news_item) = self.parse_json_news_item(item, base_url, index)? {
                    news_items.push(news_item);
                }
            }
        } else if let Some(items) = json_value.get("items").and_then(|v| v.as_array()) {
            for (index, item) in items.iter().enumerate() {
                if let Some(news_item) = self.parse_json_news_item(item, base_url, index)? {
                    news_items.push(news_item);
                }
            }
        } else if let Some(articles) = json_value.get("articles").and_then(|v| v.as_array()) {
            for (index, item) in articles.iter().enumerate() {
                if let Some(news_item) = self.parse_json_news_item(item, base_url, index)? {
                    news_items.push(news_item);
                }
            }
        } else if let Some(list) = json_value.as_array() {
            for (index, item) in list.iter().enumerate() {
                if let Some(news_item) = self.parse_json_news_item(item, base_url, index)? {
                    news_items.push(news_item);
                }
            }
        } else {
            return Box::pin(self.use_ai_for_json_parsing(json_value, base_url)).await;
        }

        Ok(news_items)
    }

    fn parse_json_news_item(&self, item: &serde_json::Value, base_url: &str, index: usize) -> Result<Option<NewsItem>> {
        let title = item.get("title")
            .or_else(|| item.get("name"))
            .or_else(|| item.get("headline"))
            .and_then(|v| v.as_str())
            .unwrap_or("无标题")
            .to_string();

        let url = item.get("url")
            .or_else(|| item.get("link"))
            .or_else(|| item.get("permalink"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let description = item.get("description")
            .or_else(|| item.get("summary"))
            .or_else(|| item.get("content"))
            .or_else(|| item.get("excerpt"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let author = item.get("author")
            .or_else(|| item.get("user"))
            .or_else(|| item.get("creator"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let timestamp = item.get("published_at")
            .or_else(|| item.get("created_at"))
            .or_else(|| item.get("date"))
            .or_else(|| item.get("timestamp"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if !title.is_empty() && !url.is_empty() {
            Ok(Some(NewsItem {
                id: format!("item_{}", index),
                title,
                url,
                desc: if description.is_empty() { None } else { Some(description) },
                author,
                timestamp,
                hot: None,
                cover: None,
                mobile_url: None,
            }))
        } else {
            Ok(None)
        }
    }

    async fn use_ai_for_json_parsing(&self, json_value: &serde_json::Value, base_url: &str) -> Result<Vec<NewsItem>> {
        let json_str = serde_json::to_string(json_value)
            .map_err(|e| anyhow!("Failed to serialize JSON for AI parsing: {}", e))?;

        let prompt = format!(
            r#"Please strictly follow the rules below to convert the provided JSON data to the specified data structure.

### 1. Target Data Structure (Schema)
Please format the output as a JSON Array, each object must contain the following fields:
- id: String (required)
- title: String (required)
- desc: String or null (optional)
- cover: String or null (optional)
- author: String or null (optional)
- timestamp: String or null (optional)
- hot: Integer or null (optional)
- url: String (required)
- mobile_url: String or null (optional)

### 2. Field Mapping & Transformation Rules
Please map the source data fields to target fields according to the table below. If the source data doesn't have a corresponding field, fill with null.

| Target Field | Source Field | Special Processing |
| :--- | :--- | :--- |
| **id** | `id` or `item_id` or `_id` | Direct string copy, if none exists generate UUID |
| **title** | `title` or `name` or `headline` | Direct string copy |
| **desc** | `description` or `summary` or `content` or `excerpt` | If source field is empty or doesn't exist, output null |
| **cover** | `cover` or `image` or `thumbnail` or `author_avatar` | If source field is empty or doesn't exist, output null |
| **author** | `author` or `user` or `creator` | Direct string copy |
| **timestamp** | `timestamp` or `created_at` or `updated_at` or `published_at` or `date` | Keep original time string format (ISO 8601) |
| **hot** | `hot` or `clicks` or `views` or `likes` or `clicks_total` | Must convert to Integer, cannot be string |
| **url** | `url` or `link` or `permalink` or `full_name` | If full_name, prepend with "https://github.com/" |
| **mobile_url** | *(none)* | Force output null (source data has no such field) |

### 3. Strict Output Requirements
- **Only output** the converted JSON array code block.
- **Prohibit** outputting any Rust code, explanatory text, Markdown format descriptions, or other irrelevant characters.
- Ensure JSON format is legal, boolean values use lowercase (true/false), null values use null.

### 4. Data to Process
{}

Please output the converted JSON array:"#,
            json_str
        );

        let response = self.call_ai_provider(&prompt).await?;

        let cleaned_response = response.trim()
            .strip_prefix("```json")
            .unwrap_or(&response)
            .strip_suffix("```")
            .unwrap_or(&response)
            .trim();

        let response_json: serde_json::Value = serde_json::from_str(cleaned_response)
            .map_err(|e| anyhow!("Failed to parse AI response as JSON: {}", e))?;

        if let Some(items) = response_json.as_array() {
            let mut news_items = Vec::new();
            for (index, item) in items.iter().enumerate() {
                if let Some(news_item) = self.parse_json_news_item(item, base_url, index)? {
                    news_items.push(news_item);
                }
            }
            Ok(news_items)
        } else {
            Err(anyhow!("AI response does not contain valid news items"))
        }
    }

    // ==================== Shared AI Provider Calls ====================

    async fn call_ai_provider(&self, prompt: &str) -> Result<String> {
        match self.config.current_config.provider.as_str() {
            "openai" | "deepseek" | "moonshot" | "qwen" | "baichuan" | "doubao" => {
                self.call_openai_compatible(prompt).await
            }
            "anthropic" => {
                self.call_anthropic(prompt).await
            }
            "zhipuai" | "chatglm" => {
                self.call_zhipuai(prompt).await
            }
            _ => Err(anyhow!("Unsupported AI provider: {}", self.config.current_config.provider))
        }
    }

    async fn call_openai_compatible(&self, prompt: &str) -> Result<String> {
        info!("Making OpenAI-compatible API call, model: {}", self.config.current_config.model);
        info!("Prompt length: {} chars", prompt.len());

        let request_body = json!({
            "model": self.config.current_config.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 4000,
            "temperature": 0.7
        });

        let api_base = self.config.current_config.api_base.as_deref().unwrap_or("https://api.openai.com/v1");
        let api_url = if api_base.ends_with("/chat/completions") {
            api_base.to_string()
        } else if api_base.ends_with("/") {
            format!("{}chat/completions", api_base)
        } else {
            format!("{}/chat/completions", api_base)
        };

        let response = self.client
            .post(&api_url)
            .header("Authorization", format!("Bearer {}", self.config.current_config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| anyhow!("OpenAI API request failed: {}", e))?;

        info!("Response status: {}", response.status());
        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read OpenAI response: {}", e))?;

        info!("Response length: {} chars", response_text.len());

        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse OpenAI response as JSON: {}. Response: {}", e, &response_text[..response_text.len().min(500)]))?;

        if let Some(content) = response_json.get("choices").and_then(|c| c.get(0)).and_then(|c| c.get("message")).and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
            Ok(content.to_string())
        } else {
            Err(anyhow!("OpenAI response does not contain valid content. Response: {}", &response_text[..response_text.len().min(500)]))
        }
    }

    async fn call_anthropic(&self, prompt: &str) -> Result<String> {
        let request_body = json!({
            "model": self.config.current_config.model,
            "max_tokens": 4000,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let api_base = self.config.current_config.api_base.as_deref().unwrap_or("https://api.anthropic.com/v1");
        let api_url = format!("{}/messages", api_base);

        let response = self.client
            .post(&api_url)
            .header("x-api-key", self.config.current_config.api_key.clone())
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| anyhow!("Anthropic API request failed: {}", e))?;

        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read Anthropic response: {}", e))?;

        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse Anthropic response as JSON: {}", e))?;

        if let Some(content) = response_json.get("content").and_then(|c| c.get(0)).and_then(|c| c.get("text")).and_then(|t| t.as_str()) {
            Ok(content.to_string())
        } else {
            Err(anyhow!("Anthropic response does not contain valid content"))
        }
    }

    async fn call_zhipuai(&self, prompt: &str) -> Result<String> {
        let request_body = json!({
            "model": self.config.current_config.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 4000,
            "temperature": 0.7
        });

        let api_base = self.config.current_config.api_base.as_deref().unwrap_or("https://open.bigmodel.cn/api/paas/v4");
        let api_url = format!("{}/chat/completions", api_base);

        let response = self.client
            .post(&api_url)
            .header("Authorization", format!("Bearer {}", self.config.current_config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| anyhow!("ZhipuAI API request failed: {}", e))?;

        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read ZhipuAI response: {}", e))?;

        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse ZhipuAI response as JSON: {}", e))?;

        if let Some(content) = response_json.get("choices").and_then(|c| c.get(0)).and_then(|c| c.get("message")).and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
            Ok(content.to_string())
        } else {
            Err(anyhow!("ZhipuAI response does not contain valid content"))
        }
    }

    // ==================== Utility Methods ====================

    pub async fn test_connection(&self, test_prompt: &str) -> Result<String> {
        match self.config.current_config.provider.to_lowercase().as_str() {
            "openai" | "deepseek" | "moonshot" | "qwen" | "baichuan" | "doubao" => {
                self.call_openai_compatible(test_prompt).await
            }
            "anthropic" => {
                self.call_anthropic(test_prompt).await
            }
            "zhipuai" | "chatglm" => {
                self.call_zhipuai(test_prompt).await
            }
            _ => self.call_openai_compatible(test_prompt).await,
        }
    }

    pub fn get_provider_name(&self) -> String {
        self.config.current_config.provider.clone()
    }

    pub fn get_model_name(&self) -> String {
        self.config.current_config.model.clone()
    }

    pub fn is_enabled(&self) -> bool {
        self.config.current_config.enabled
    }
}
