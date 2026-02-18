use sandy::tools::memory_log::MemoryLogTool;
use sandy::tools::tool_filter;
use sandy::tools::Tool;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    println!("=== Testing Guardrails ===\n");

    // Test 1: Tool Filter - Forbidden tools
    println!("Test 1: Try to use web_search (should be blocked)");
    match tool_filter::can_sandy_use("web_search") {
        Ok(_) => println!("  ❌ FAIL: Should have been blocked"),
        Err(msg) => {
            println!("  ✅ PASS: Blocked as expected");
            println!("  Message: {}", msg);
        }
    }
    println!();

    // Test 2: Tool Filter - Allowed tools
    println!("Test 2: Try to use schedule_task (should be allowed)");
    match tool_filter::can_sandy_use("schedule_task") {
        Ok(_) => println!("  ✅ PASS: Allowed as expected"),
        Err(msg) => println!("  ❌ FAIL: Should have been allowed. Error: {}", msg),
    }
    println!();

    // Test 3: Memory Log - No verification (should fail)
    println!("Test 3: Log solution WITHOUT verification (should be rejected)");
    let temp_dir = std::env::temp_dir().join("sandy_test_guardrails");
    let log_tool = MemoryLogTool::new(temp_dir.clone());

    let params = serde_json::json!({
        "category": "solutions",
        "content": "Fixed the scheduler by updating configuration"
    });

    let result = log_tool.execute(params).await;
    if result.is_error {
        println!("  ✅ PASS: Rejected as expected");
        println!("  Error: {}", result.content);
    } else {
        println!("  ❌ FAIL: Should have been rejected");
    }
    println!();

    // Test 4: Memory Log - WITH verification (should succeed)
    println!("Test 4: Log solution WITH verification (should be accepted)");
    let params = serde_json::json!({
        "category": "solutions",
        "content": "Fixed scheduler by updating AGENTS.md line 25 to use correct tool name",
        "verification": "Verified with: grep list_scheduled_tasks AGENTS.md shows correct name. Tested reminder, it fired successfully."
    });

    let result = log_tool.execute(params).await;
    if !result.is_error {
        println!("  ✅ PASS: Accepted with verification");
        println!("  Result: {}", result.content);

        // Check file content
        let file = temp_dir.join("solutions.md");
        if file.exists() {
            let content = std::fs::read_to_string(&file).unwrap();
            if content.contains("**Verification:**") {
                println!("  ✅ PASS: Verification included in file");
            } else {
                println!("  ❌ FAIL: Verification not in file");
            }
        }
    } else {
        println!("  ❌ FAIL: Should have been accepted");
        println!("  Error: {}", result.content);
    }
    println!();

    // Test 5: Memory Log - Vague content (should fail)
    println!("Test 5: Log with vague content (should be rejected)");
    let params = serde_json::json!({
        "category": "solutions",
        "content": "Fixed it",
        "verification": "Works"
    });

    let result = log_tool.execute(params).await;
    if result.is_error {
        println!("  ✅ PASS: Vague content rejected");
        println!("  Error: {}", result.content);
    } else {
        println!("  ❌ FAIL: Vague content should be rejected");
    }
    println!();

    // Test 6: Memory Log - Too short (should fail)
    println!("Test 6: Log with content too short (should be rejected)");
    let params = serde_json::json!({
        "category": "errors",
        "content": "Error occurred"
    });

    let result = log_tool.execute(params).await;
    if result.is_error && result.content.contains("too short") {
        println!("  ✅ PASS: Short content rejected");
        println!("  Error: {}", result.content);
    } else {
        println!("  ❌ FAIL: Short content should be rejected");
    }
    println!();

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);

    println!("=== Guardrails Test Complete ===");
}
