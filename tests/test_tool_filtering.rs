// Test tool filtering for agent execution
// Ensures agents only get their whitelisted tools

use sandy::config::Config;
use sandy::tools::{Tool, ToolRegistry};
use std::collections::HashSet;

fn test_config() -> Config {
    Config {
        telegram_bot_token: "tok".into(),
        bot_username: "bot".into(),
        llm_provider: "anthropic".into(),
        api_key: "key".into(),
        model: "claude-test".into(),
        llm_base_url: None,
        max_tokens: 4096,
        max_tool_iterations: 100,
        max_sub_agent_iterations: 25,
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
        fallback_models: vec![],
        tavily_api_key: None,
        web_port: 3000,
        soul_file: "soul/SOUL.md".into(),
        identity_file: "soul/IDENTITY.md".into(),
        agents_file: "soul/AGENTS.md".into(),
    }
}

#[test]
fn test_empty_tool_list_creates_empty_registry() {
    use sandy::tools::agent_management::SpawnAgentTool;

    let config = test_config();
    let spawn_tool = SpawnAgentTool::new(&config);

    // Use reflection to access private method (for testing)
    // In real code, this would be tested through public API
    let empty_tools: Vec<String> = vec![];

    // Create filtered registry via the tool's internal logic
    // This tests that an empty allowed list results in no tools
    let method = spawn_tool.create_filtered_registry(&empty_tools);

    assert_eq!(
        method.tool_count(),
        0,
        "Empty tool list should create registry with 0 tools"
    );
}

#[test]
fn test_single_tool_filter() {
    use sandy::tools::agent_management::SpawnAgentTool;

    let config = test_config();
    let spawn_tool = SpawnAgentTool::new(&config);

    let allowed_tools = vec!["read_file".to_string()];
    let filtered = spawn_tool.create_filtered_registry(&allowed_tools);

    assert_eq!(filtered.tool_count(), 1, "Should have exactly 1 tool");
    assert!(
        filtered.has_tool("read_file"),
        "Should have read_file tool"
    );
    assert!(
        !filtered.has_tool("write_file"),
        "Should NOT have write_file"
    );
    assert!(!filtered.has_tool("web_search"), "Should NOT have web_search");
}

#[test]
fn test_zilla_tool_filtering() {
    use sandy::tools::agent_management::SpawnAgentTool;

    let config = test_config();
    let spawn_tool = SpawnAgentTool::new(&config);

    // Zilla's tools from zilla.json: web_search, web_fetch, write_file, read_file, bash
    let zilla_tools = vec![
        "web_search".to_string(),
        "web_fetch".to_string(),
        "write_file".to_string(),
        "read_file".to_string(),
        "bash".to_string(),
    ];

    let filtered = spawn_tool.create_filtered_registry(&zilla_tools);

    assert_eq!(
        filtered.tool_count(),
        5,
        "Zilla should have exactly 5 tools"
    );

    // Should have
    assert!(filtered.has_tool("web_search"), "Zilla should have web_search");
    assert!(filtered.has_tool("web_fetch"), "Zilla should have web_fetch");
    assert!(filtered.has_tool("write_file"), "Zilla should have write_file");
    assert!(filtered.has_tool("read_file"), "Zilla should have read_file");
    assert!(filtered.has_tool("bash"), "Zilla should have bash");

    // Should NOT have
    assert!(
        !filtered.has_tool("spawn_agent"),
        "Zilla should NOT have spawn_agent"
    );
    assert!(
        !filtered.has_tool("send_message"),
        "Zilla should NOT have send_message"
    );
    assert!(
        !filtered.has_tool("edit_file"),
        "Zilla should NOT have edit_file (not in allowed list)"
    );
}

#[test]
fn test_gonza_tool_filtering() {
    use sandy::tools::agent_management::SpawnAgentTool;

    let config = test_config();
    let spawn_tool = SpawnAgentTool::new(&config);

    // Gonza's tools from gonza.json: read_file, write_file (NO web access)
    let gonza_tools = vec!["read_file".to_string(), "write_file".to_string()];

    let filtered = spawn_tool.create_filtered_registry(&gonza_tools);

    assert_eq!(
        filtered.tool_count(),
        2,
        "Gonza should have exactly 2 tools"
    );

    // Should have
    assert!(filtered.has_tool("read_file"), "Gonza should have read_file");
    assert!(
        filtered.has_tool("write_file"),
        "Gonza should have write_file"
    );

    // Should NOT have (Gonza cannot access web)
    assert!(
        !filtered.has_tool("web_search"),
        "Gonza should NOT have web_search"
    );
    assert!(
        !filtered.has_tool("web_fetch"),
        "Gonza should NOT have web_fetch"
    );
    assert!(!filtered.has_tool("bash"), "Gonza should NOT have bash");
    assert!(
        !filtered.has_tool("spawn_agent"),
        "Gonza should NOT have spawn_agent"
    );
}

#[test]
fn test_all_allowed_tools() {
    use sandy::tools::agent_management::SpawnAgentTool;

    let config = test_config();
    let spawn_tool = SpawnAgentTool::new(&config);

    let all_tools = vec![
        "bash".to_string(),
        "browser".to_string(),
        "read_file".to_string(),
        "write_file".to_string(),
        "edit_file".to_string(),
        "glob".to_string(),
        "grep".to_string(),
        "read_memory".to_string(),
        "web_fetch".to_string(),
        "web_search".to_string(),
        "activate_skill".to_string(),
    ];

    let filtered = spawn_tool.create_filtered_registry(&all_tools);

    assert_eq!(
        filtered.tool_count(),
        11,
        "Should have all 11 sub-agent tools"
    );

    for tool_name in &all_tools {
        assert!(
            filtered.has_tool(tool_name),
            "Should have {} tool",
            tool_name
        );
    }
}

#[test]
fn test_invalid_tool_name_ignored() {
    use sandy::tools::agent_management::SpawnAgentTool;

    let config = test_config();
    let spawn_tool = SpawnAgentTool::new(&config);

    let tools_with_invalid = vec![
        "read_file".to_string(),
        "invalid_tool".to_string(), // This doesn't exist
        "write_file".to_string(),
    ];

    let filtered = spawn_tool.create_filtered_registry(&tools_with_invalid);

    // Should only have 2 valid tools (invalid_tool ignored)
    assert_eq!(
        filtered.tool_count(),
        2,
        "Invalid tool names should be ignored"
    );
    assert!(filtered.has_tool("read_file"));
    assert!(filtered.has_tool("write_file"));
    assert!(!filtered.has_tool("invalid_tool"));
}

#[test]
fn test_tool_registry_tool_names() {
    use sandy::tools::agent_management::SpawnAgentTool;

    let config = test_config();
    let spawn_tool = SpawnAgentTool::new(&config);

    let allowed = vec!["bash".to_string(), "read_file".to_string()];
    let filtered = spawn_tool.create_filtered_registry(&allowed);

    let names = filtered.tool_names();
    let names_set: HashSet<String> = names.into_iter().collect();

    assert_eq!(names_set.len(), 2);
    assert!(names_set.contains("bash"));
    assert!(names_set.contains("read_file"));
}

#[test]
fn test_sub_agent_tool_with_custom_registry() {
    use sandy::tools::sub_agent::SubAgentTool;
    use sandy::tools::agent_management::SpawnAgentTool;

    let config = test_config();
    let spawn_tool = SpawnAgentTool::new(&config);

    // Create a filtered registry
    let allowed = vec!["read_file".to_string()];
    let filtered = spawn_tool.create_filtered_registry(&allowed);

    // Create SubAgentTool with custom registry
    let sub_agent = SubAgentTool::with_registry(&config, filtered);

    // SubAgentTool should be created successfully
    assert_eq!(sub_agent.name(), "sub_agent");
}

#[test]
fn test_sub_agent_tool_default_registry() {
    use sandy::tools::sub_agent::SubAgentTool;

    let config = test_config();

    // Create SubAgentTool with default registry
    let sub_agent = SubAgentTool::new(&config);

    assert_eq!(sub_agent.name(), "sub_agent");
    // Default registry should have all 11 sub-agent tools
}
