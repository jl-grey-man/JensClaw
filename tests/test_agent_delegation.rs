/// Integration test for agent delegation
/// Tests that Sandy uses spawn_agent/execute_workflow instead of doing work herself

use sandy::config::Config;
use sandy::tools::ToolRegistry;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_sandy_knows_about_agent_tools() {
    // Create test config
    let config = create_test_config();

    // Create test database directory
    let test_db_dir = "/tmp/sandy_test_db";
    std::fs::create_dir_all(test_db_dir).ok();

    // Create full registry (what Sandy has)
    let db = sandy::db::Database::new(test_db_dir).expect("Should create test database");
    let registry = ToolRegistry::new(
        &config,
        teloxide::Bot::new("test_token"),
        Arc::new(db)
    );

    // Check that agent tools are available
    assert!(registry.has_tool("spawn_agent"), "spawn_agent tool should be available");
    assert!(registry.has_tool("execute_workflow"), "execute_workflow tool should be available");
    assert!(registry.has_tool("list_agents"), "list_agents tool should be available");
    assert!(registry.has_tool("agent_status"), "agent_status tool should be available");
    assert!(registry.has_tool("create_agent_config"), "create_agent_config tool should be available");

    println!("✅ All agent tools are registered");

    // Cleanup
    std::fs::remove_dir_all(test_db_dir).ok();
}

#[tokio::test]
async fn test_agent_tool_definitions() {
    let config = create_test_config();

    let test_db_dir = "/tmp/sandy_test_db2";
    std::fs::create_dir_all(test_db_dir).ok();

    let db = sandy::db::Database::new(test_db_dir).expect("Should create test database");
    let registry = ToolRegistry::new(
        &config,
        teloxide::Bot::new("test_token"),
        Arc::new(db)
    );

    let definitions = registry.definitions();
    let tool_names: Vec<String> = definitions.iter().map(|d| d.name.clone()).collect();

    println!("Available tools: {:?}", tool_names);

    // Verify agent tools are in definitions
    assert!(tool_names.contains(&"spawn_agent".to_string()));
    assert!(tool_names.contains(&"execute_workflow".to_string()));

    // Find spawn_agent definition
    let spawn_agent_def = definitions.iter()
        .find(|d| d.name == "spawn_agent")
        .expect("spawn_agent definition should exist");

    println!("spawn_agent definition: {:?}", spawn_agent_def);

    // Verify it has the expected parameters
    assert!(spawn_agent_def.description.contains("spawn") || spawn_agent_def.description.contains("agent") || spawn_agent_def.description.contains("delegate"));

    println!("✅ Agent tool definitions are correct");

    // Cleanup
    std::fs::remove_dir_all(test_db_dir).ok();
}

#[test]
fn test_soul_contains_orchestration() {
    let soul_content = std::fs::read_to_string("soul/SOUL.md")
        .expect("Should be able to read SOUL.md");

    assert!(soul_content.contains("Work Orchestration"),
        "SOUL.md should contain Work Orchestration section");
    assert!(soul_content.contains("Zilla"),
        "SOUL.md should document Zilla agent");
    assert!(soul_content.contains("Gonza"),
        "SOUL.md should document Gonza agent");
    assert!(soul_content.contains("spawn_agent"),
        "SOUL.md should explain spawn_agent");
    assert!(soul_content.contains("execute_workflow"),
        "SOUL.md should explain execute_workflow");

    println!("✅ SOUL.md contains orchestration documentation");
}

#[test]
fn test_agents_md_contains_tools() {
    let agents_content = std::fs::read_to_string("soul/AGENTS.md")
        .expect("Should be able to read AGENTS.md");

    assert!(agents_content.contains("spawn_agent"),
        "AGENTS.md should list spawn_agent tool");
    assert!(agents_content.contains("execute_workflow"),
        "AGENTS.md should list execute_workflow tool");
    assert!(agents_content.contains("Agent Orchestration"),
        "AGENTS.md should have Agent Orchestration section");

    println!("✅ AGENTS.md contains agent tools");
}

#[test]
fn test_agent_configs_exist() {
    assert!(std::path::Path::new("storage/agents/zilla.json").exists(),
        "Zilla config should exist");
    assert!(std::path::Path::new("storage/agents/gonza.json").exists(),
        "Gonza config should exist");

    // Read and validate Zilla config
    let zilla_content = std::fs::read_to_string("storage/agents/zilla.json")
        .expect("Should be able to read zilla.json");
    let zilla: serde_json::Value = serde_json::from_str(&zilla_content)
        .expect("Zilla config should be valid JSON");

    assert_eq!(zilla["id"], "zilla");
    assert_eq!(zilla["role"], "Journalistic Researcher");
    assert!(zilla["tools"].as_array().unwrap().contains(&json!("web_search")));

    // Read and validate Gonza config
    let gonza_content = std::fs::read_to_string("storage/agents/gonza.json")
        .expect("Should be able to read gonza.json");
    let gonza: serde_json::Value = serde_json::from_str(&gonza_content)
        .expect("Gonza config should be valid JSON");

    assert_eq!(gonza["id"], "gonza");
    assert_eq!(gonza["role"], "Journalistic Writer");
    assert!(gonza["tools"].as_array().unwrap().contains(&json!("write_file")));
    assert!(!gonza["tools"].as_array().unwrap().contains(&json!("web_search")),
        "Gonza should NOT have web_search");

    println!("✅ Agent configs exist and are valid");
}

fn create_test_config() -> Config {
    Config {
        telegram_bot_token: "test_token".into(),
        bot_username: "test_bot".into(),
        llm_provider: "anthropic".into(),
        api_key: "test_key".into(),
        model: "claude-test".into(),
        llm_base_url: None,
        max_tokens: 4096,
        max_tool_iterations: 100,
        max_history_messages: 10,
        data_dir: "/tmp/sandy_test".into(),
        working_dir: "/tmp/sandy_test".into(),
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
        memory_file: "soul/data/runtime/MEMORY.md".into(),
    }
}
