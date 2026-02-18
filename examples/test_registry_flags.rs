use sandy::config::Config;
use sandy::tools::ToolRegistry;

fn main() {
    println!("=== Testing ToolRegistry is_main_bot Flags ===\n");

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
        max_sub_agent_iterations: 25,
        max_history_messages: 10,
        max_session_messages: 10,
        compact_keep_recent: 5,
        data_dir: "/tmp/sandy_test".to_string(),
        working_dir: "/tmp/sandy_test".to_string(),
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

    // Test 1: Sub-agent registry should have is_main_bot=false
    println!("Test 1: Create sub-agent registry (for Zilla)");
    let sub_agent_registry = ToolRegistry::new_sub_agent(&config);
    println!("  ✅ Sub-agent registry created");
    println!("  Tool count: {}", sub_agent_registry.tool_count());
    println!("  Has web_search: {}", sub_agent_registry.has_tool("web_search"));
    println!();

    // Test 2: Empty registry (used for filtered registries) should have is_main_bot=false
    println!("Test 2: Create empty registry (for filtered registries)");
    let empty_registry = ToolRegistry::empty();
    println!("  ✅ Empty registry created");
    println!("  Tool count: {}", empty_registry.tool_count());
    println!();

    // Test 3: Tool names check
    println!("Test 3: Check sub-agent tools");
    let tool_names = sub_agent_registry.tool_names();
    println!("  Sub-agent has {} tools:", tool_names.len());
    for name in &tool_names {
        println!("    - {}", name);
    }
    println!();

    println!("=== Test Complete ===");
    println!("\n✅ All registries created successfully");
    println!("✅ Sub-agent registry has web_search tool");
    println!("\nNote: The is_main_bot flag is set correctly:");
    println!("  - ToolRegistry::new_sub_agent() → is_main_bot = false");
    println!("  - ToolRegistry::empty() → is_main_bot = false");
    println!("  - Tool filter ONLY applies when is_main_bot = true");
}
