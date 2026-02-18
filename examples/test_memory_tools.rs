use sandy::tools::memory_search::MemorySearchTool;
use sandy::tools::memory_log::MemoryLogTool;
use sandy::tools::Tool;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    println!("=== Testing Memory Tools ===\n");

    // Test 1: Memory Search
    println!("Test 1: Search for 'scheduler'");
    let search_tool = MemorySearchTool::new(PathBuf::from("soul/data/memory"));
    let params = serde_json::json!({
        "query": "scheduler",
        "limit": 2
    });

    let result = search_tool.execute(params).await;
    println!("  Result: {}", if result.is_error { "ERROR" } else { "SUCCESS" });
    println!("  Content: {}\n", result.content);

    // Test 2: Search for 'permission'
    println!("Test 2: Search for 'permission'");
    let params = serde_json::json!({
        "query": "permission"
    });

    let result = search_tool.execute(params).await;
    println!("  Result: {}", if result.is_error { "ERROR" } else { "SUCCESS" });
    println!("  Content: {}\n", result.content);

    // Test 3: Search for non-existent
    println!("Test 3: Search for 'nonexistent'");
    let params = serde_json::json!({
        "query": "nonexistent_term_12345"
    });

    let result = search_tool.execute(params).await;
    println!("  Result: {}", if result.is_error { "ERROR" } else { "SUCCESS" });
    println!("  Content: {}\n", result.content);

    // Test 4: Memory Log (to errors.md)
    println!("Test 4: Log to errors.md");
    let log_tool = MemoryLogTool::new(PathBuf::from("soul/data/memory"));
    let params = serde_json::json!({
        "category": "errors",
        "content": "Test error: This is a test log entry to verify log_memory tool works correctly."
    });

    let result = log_tool.execute(params).await;
    println!("  Result: {}", if result.is_error { "ERROR" } else { "SUCCESS" });
    println!("  Content: {}\n", result.content);

    // Test 5: Verify the log was written
    println!("Test 5: Verify errors.md was created");
    let errors_file = PathBuf::from("soul/data/memory/errors.md");
    if errors_file.exists() {
        let content = std::fs::read_to_string(&errors_file).unwrap();
        println!("  ✅ File exists");
        println!("  Content preview: {}\n", content.chars().take(200).collect::<String>());
    } else {
        println!("  ❌ File not created\n");
    }

    // Test 6: Invalid category
    println!("Test 6: Try invalid category");
    let params = serde_json::json!({
        "category": "invalid",
        "content": "Should fail"
    });

    let result = log_tool.execute(params).await;
    println!("  Result: {}", if result.is_error { "ERROR (expected)" } else { "SUCCESS (unexpected)" });
    println!("  Content: {}\n", result.content);

    println!("=== All Tests Complete ===");
}
