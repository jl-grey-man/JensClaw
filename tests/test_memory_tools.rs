use sandy::tools::memory_search::MemorySearchTool;
use sandy::tools::memory_log::MemoryLogTool;
use sandy::tools::Tool;
use std::path::PathBuf;

#[tokio::test]
async fn test_memory_search_existing_content() {
    let tool = MemorySearchTool::new(PathBuf::from("soul/data/memory"));

    let params = serde_json::json!({
        "query": "scheduler",
        "limit": 3
    });

    let result = tool.execute(params).await;
    assert!(!result.is_error, "Should not error: {}", result.content);
    assert!(result.content.contains("Found") || result.content.contains("No memories"),
            "Should return search results: {}", result.content);
}

#[tokio::test]
async fn test_memory_search_nonexistent() {
    let tool = MemorySearchTool::new(PathBuf::from("soul/data/memory"));

    let params = serde_json::json!({
        "query": "this_definitely_does_not_exist_anywhere_12345",
        "limit": 3
    });

    let result = tool.execute(params).await;
    assert!(!result.is_error, "Should not error even if nothing found");
    assert!(result.content.contains("No memories found"),
            "Should indicate no results: {}", result.content);
}

#[tokio::test]
async fn test_memory_search_missing_query() {
    let tool = MemorySearchTool::new(PathBuf::from("soul/data/memory"));

    let params = serde_json::json!({
        "limit": 3
    });

    let result = tool.execute(params).await;
    assert!(result.is_error, "Should error when query is missing");
    assert!(result.content.contains("Missing 'query'"),
            "Should indicate missing parameter: {}", result.content);
}

#[tokio::test]
async fn test_memory_log_success() {
    // Use temp directory for testing
    let temp_dir = std::env::temp_dir().join(format!("sandy_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let tool = MemoryLogTool::new(temp_dir.clone());

    let params = serde_json::json!({
        "category": "solutions",
        "content": "Test solution: this is a test entry"
    });

    let result = tool.execute(params).await;
    assert!(!result.is_error, "Should successfully log: {}", result.content);
    assert!(result.content.contains("Recorded to solutions.md"),
            "Should confirm recording: {}", result.content);

    // Verify file was created
    let file_path = temp_dir.join("solutions.md");
    assert!(file_path.exists(), "solutions.md should be created");

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("Test solution"), "Should contain logged content");
    assert!(content.contains("##"), "Should have timestamp header");

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_memory_log_invalid_category() {
    let temp_dir = std::env::temp_dir().join(format!("sandy_test_{}", uuid::Uuid::new_v4()));
    let tool = MemoryLogTool::new(temp_dir);

    let params = serde_json::json!({
        "category": "invalid_category",
        "content": "Test content"
    });

    let result = tool.execute(params).await;
    assert!(result.is_error, "Should error with invalid category");
    assert!(result.content.contains("must be one of"),
            "Should indicate valid categories: {}", result.content);
}

#[tokio::test]
async fn test_memory_log_missing_parameters() {
    let temp_dir = std::env::temp_dir().join(format!("sandy_test_{}", uuid::Uuid::new_v4()));
    let tool = MemoryLogTool::new(temp_dir);

    // Missing content
    let params = serde_json::json!({
        "category": "solutions"
    });

    let result = tool.execute(params).await;
    assert!(result.is_error, "Should error when content is missing");
    assert!(result.content.contains("Missing 'content'"),
            "Should indicate missing parameter: {}", result.content);
}
