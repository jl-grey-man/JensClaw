use async_trait::async_trait;
use serde_json::json;
use std::time::Duration;
use tavily::{Tavily, SearchRequest};

use super::{schema_object, Tool, ToolResult};
use crate::claude::ToolDefinition;
use crate::config::Config;

pub struct WebSearchTool {
    config: Config,
}

impl WebSearchTool {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    fn get_api_key(&self) -> Result<String, String> {
        self.config
            .tavily_api_key
            .clone()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| "Tavily API key not configured. Add tavily_api_key to your sandy.config.yaml file.".to_string())
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_search".into(),
            description: "Search the web using Tavily API. Returns structured results with titles, URLs, content, and source quality scores."
                .into(),
            input_schema: schema_object(
                json!({
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    },
                    "search_depth": {
                        "type": "string",
                        "description": "Search depth: 'basic' (fast) or 'advanced' (thorough). Default: 'basic'.",
                        "enum": ["basic", "advanced"],
                        "default": "basic"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results to return (1-20). Default: 5.",
                        "minimum": 1,
                        "maximum": 20,
                        "default": 5
                    },
                    "include_domains": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Optional: List of domains to include in search (e.g., ['github.com', 'reddit.com'])",
                        "default": []
                    },
                    "exclude_domains": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Optional: List of domains to exclude from search",
                        "default": []
                    }
                }),
                &["query"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let query = match input.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return ToolResult::error("Missing required parameter: query".into()),
        };

        let search_depth = input.get("search_depth")
            .and_then(|v| v.as_str())
            .unwrap_or("basic");

        let max_results = input.get("max_results")
            .and_then(|v| v.as_i64())
            .unwrap_or(5) as i32;

        let include_domains: Vec<String> = input.get("include_domains")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|val| val.as_str().map(|s| s.to_string()))
                .collect())
            .unwrap_or_default();

        let exclude_domains: Vec<String> = input.get("exclude_domains")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|val| val.as_str().map(|s| s.to_string()))
                .collect())
            .unwrap_or_default();

        let api_key = match self.get_api_key() {
            Ok(key) => key,
            Err(e) => return ToolResult::error(e),
        };

        match search_tavily(&api_key, query, search_depth, max_results, include_domains, exclude_domains).await {
            Ok(results) => ToolResult::success(results),
            Err(e) => ToolResult::error(format!("Search failed: {}", e)),
        }
    }
}

async fn search_tavily(
    api_key: &str,
    query: &str,
    search_depth: &str,
    max_results: i32,
    include_domains: Vec<String>,
    exclude_domains: Vec<String>,
) -> Result<String, String> {
    // Build Tavily client
    let tavily = Tavily::builder(api_key)
        .timeout(Duration::from_secs(60))
        .max_retries(3)
        .build()
        .map_err(|e| format!("Failed to build Tavily client: {}", e))?;

    // Build search request with chained builder pattern
    let mut request = SearchRequest::new(api_key, query)
        .search_depth(search_depth)
        .max_results(max_results);

    // Add domain filters if specified
    if !include_domains.is_empty() {
        request = request.include_domains(&include_domains);
    }
    if !exclude_domains.is_empty() {
        request = request.exclude_domains(&exclude_domains);
    }

    // Execute search
    let response = tavily.call(&request)
        .await
        .map_err(|e| format!("Tavily API error: {}", e))?;

    Ok(format_response(&response))
}

fn format_response(response: &tavily::SearchResponse) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("Query: {}\n", response.query));
    
    if let Some(ref answer) = response.answer {
        output.push_str(&format!("Answer: {}\n\n", answer));
    }
    
    output.push_str(&format!("Results ({} total):\n\n", response.results.len()));

    for (i, result) in response.results.iter().enumerate() {
        output.push_str(&format!(
            "{}. {}\n   URL: {}\n   Content: {}\n   Score: {:.2}\n\n",
            i + 1,
            result.title,
            result.url,
            result.content.chars().take(300).collect::<String>(),
            result.score
        ));
    }

    if let Some(ref images) = response.images {
        if !images.is_empty() {
            output.push_str(&format!("\nImages ({} found):\n", images.len()));
            for (i, img) in images.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", i + 1, img));
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_config() -> Config {
        Config {
            telegram_bot_token: "tok".into(),
            bot_username: "bot".into(),
            llm_provider: "anthropic".into(),
            api_key: "key".into(),
            model: "claude-test".into(),
            fallback_models: vec![],
            llm_base_url: None,
            max_tokens: 4096,
            max_tool_iterations: 100,
            max_history_messages: 10,
            data_dir: "/tmp".into(),
            working_dir: "/tmp".into(),
            openai_api_key: None,
            timezone: "UTC".into(),
            allowed_groups: vec![],
            control_chat_ids: vec![],
            max_session_messages: 25,
            compact_keep_recent: 10,
            whatsapp_access_token: None,
            whatsapp_phone_number_id: None,
            whatsapp_verify_token: None,
            whatsapp_webhook_port: 8080,
            discord_bot_token: None,
            discord_allowed_channels: vec![],
            show_thinking: false,
            tavily_api_key: None,
            web_port: 3000,
            soul_file: "soul/SOUL.md".into(),
            identity_file: "soul/IDENTITY.md".into(),
            agents_file: "soul/AGENTS.md".into(),
            memory_file: "soul/data/MEMORY.md".into(),
        }
    }

    #[test]
    fn test_web_search_definition() {
        let tool = WebSearchTool::new(&test_config());
        assert_eq!(tool.name(), "web_search");
        let def = tool.definition();
        assert_eq!(def.name, "web_search");
        assert!(def.description.contains("Tavily"));
        assert!(def.input_schema["properties"]["query"].is_object());
        assert!(def.input_schema["properties"]["search_depth"].is_object());
        assert!(def.input_schema["properties"]["max_results"].is_object());
        let required = def.input_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "query"));
    }

    #[tokio::test]
    async fn test_web_search_missing_query() {
        let tool = WebSearchTool::new(&test_config());
        let result = tool.execute(json!({})).await;
        assert!(result.is_error);
        assert!(result.content.contains("Missing required parameter: query"));
    }

    #[tokio::test]
    async fn test_web_search_null_query() {
        let tool = WebSearchTool::new(&test_config());
        let result = tool.execute(json!({"query": null})).await;
        assert!(result.is_error);
        assert!(result.content.contains("Missing required parameter: query"));
    }

    #[test]
    fn test_api_key_not_set() {
        let config = test_config();
        let tool = WebSearchTool::new(&config);
        let result = tool.get_api_key();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Tavily API key not configured"));
    }
}
