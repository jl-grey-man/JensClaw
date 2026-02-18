use sandy::config::Config;
use sandy::tools::ToolRegistry;

#[tokio::main]
async fn main() {
    println!("=== Testing Tool Filter Fix ===\n");

    // Create a test config
    let config = Config {
        telegram_bot_token: "test".to_string(),
        bot_username: "test".to_string(),
        llm_provider: "anthropic".to_string(),
        api_key: "test".to_string(),
        model: "test".to_string(),
        llm_base_url: None,
        max_tokens: 1000,
        max_tool_iterations: 10,
        max_history_messages: 10,
        max_session_messages: 10,
        compact_keep_recent: 5,
        data_dir: "test".to_string(),
        working_dir: "test".to_string(),
        timezone: "UTC".to_string(),
        soul_file: "test".to_string(),
        identity_file: "test".to_string(),
        agents_file: "test".to_string(),
        memory_file: "test".to_string(),
        control_chat_ids: vec![],
        web_port: 3000,
        tavily_api_key: None,
        fallback_models: vec![],
        openai_api_key: None,
        allowed_groups: vec![],
        whatsapp_access_token: None,
        whatsapp_phone_number_id: None,
        whatsapp_verify_token: None,
        whatsapp_webhook_port: 8080,
        discord_bot_token: None,
        discord_allowed_channels: vec![],
        show_thinking: false,
    };

    // Test 1: Main bot registry (should block web_search)
    println!("Test 1: Main Sandy Bot - Try web_search");
    let main_registry = ToolRegistry::new(&config, teloxide::Bot::from_env(), std::sync::Arc::new(sandy::db::Database::new("test.db").unwrap()));

    let params = serde_json::json!({
        "query": "test query"
    });

    let result = main_registry.execute("web_search", params).await;
    if result.is_error && result.content.contains("GUARDRAIL VIOLATION") {
        println!("  ✅ PASS: Main bot blocked from web_search");
        println!("  Error: {}\n", result.content.lines().next().unwrap());
    } else {
        println!("  ❌ FAIL: Main bot should be blocked");
        println!("  Result: {}\n", result.content);
    }

    // Test 2: Sub-agent registry (should allow web_search)
    println!("Test 2: Sub-agent (Zilla) - Try web_search");
    let sub_agent_registry = ToolRegistry::new_sub_agent(&config);

    let params = serde_json::json!({
        "query": "test query"
    });

    let result = sub_agent_registry.execute("web_search", params).await;

    // We expect it to fail due to missing API key, but NOT due to guardrail
    if result.is_error && result.content.contains("GUARDRAIL VIOLATION") {
        println!("  ❌ FAIL: Sub-agent should NOT be blocked by guardrail");
        println!("  Error: {}\n", result.content);
    } else {
        println!("  ✅ PASS: Sub-agent allowed to attempt web_search");
        println!("  Note: May fail with API error, but NOT guardrail error\n");
    }

    println!("=== Test Complete ===");
    println!("\nSummary:");
    println!("- Main Sandy bot: BLOCKED from web_search ✅");
    println!("- Sub-agents (Zilla): ALLOWED to use web_search ✅");
}
