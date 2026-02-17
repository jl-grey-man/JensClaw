# Sandy Self-Healing: Atomic Implementation Plan
**Date:** 2026-02-16
**Strategy:** Small, testable increments. Each task is independently deployable.

---

## Priority 1: Foundation Tasks (Start Here)

These provide immediate value and enable later features.

---

### Task 1: Add Task Complexity Keywords to SOUL.md
**Time:** 30 minutes
**Value:** High - Immediate behavior improvement
**Dependencies:** None

**Objective:**
Update Sandy's system prompt so she recognizes when to delegate autonomously.

**Files to Modify:**
- `soul/SOUL.md`

**Changes:**
Add after "Work Orchestration" section:
```markdown
## Autonomous Task Recognition

**BEFORE responding to any request, assess:**

### Research Tasks → Spawn Zilla Immediately
Keywords: research, find information, what's happening, latest news, search for, look up, investigate

Example:
- "What's happening in AI?" → spawn Zilla
- "Research ADHD tools" → spawn Zilla
- "Find the latest news about X" → spawn Zilla

### Writing Tasks → Spawn Gonza Immediately
Keywords: write article, create summary, draft document, compose, generate report

Example:
- "Write an article about X" → spawn Gonza
- "Create a summary of Y" → spawn Gonza

### Combined Tasks → Use execute_workflow
Keywords: research AND write, find AND summarize, investigate AND report

Example:
- "Research AI and write summary" → execute_workflow([Zilla, Gonza])

**Key Rule:** Don't wait for user to say "use Zilla". When you see research keywords, automatically delegate.
```

**Testing:**
```bash
# Rebuild and restart Sandy
cargo build --release && sudo systemctl restart sandy

# Test via Telegram:
1. "What's happening in quantum computing?"
   Expected: Sandy spawns Zilla automatically (not manual web_search)

2. "Write a poem about cats"
   Expected: Sandy spawns Gonza automatically

3. "Research AI and write an article"
   Expected: Sandy uses execute_workflow
```

**Success Criteria:**
- [ ] Sandy spawns Zilla when user asks research questions
- [ ] No need for explicit "use Zilla" command
- [ ] Logs show `spawn_agent` tool calls

---

### Task 2: Add Memory Search Tool
**Time:** 2 hours
**Value:** High - Enables learning from past
**Dependencies:** None

**Objective:**
Let Sandy search her memory files before attempting solutions.

**Files to Create:**
- `src/tools/memory_search.rs`

**Implementation:**
```rust
use crate::tools::Tool;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub struct MemorySearchTool {
    memory_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct MemorySearchParams {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize { 5 }

impl MemorySearchTool {
    pub fn new(memory_dir: PathBuf) -> Self {
        Self { memory_dir }
    }
}

#[async_trait::async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str { "search_memory" }

    fn description(&self) -> &str {
        "Search past memories, solutions, and error patterns. Use this before attempting to solve a problem to see if you've solved it before."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What to search for (error message, topic, solution keyword)"
                },
                "limit": {
                    "type": "number",
                    "description": "Max results to return (default: 5)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let params: MemorySearchParams = serde_json::from_value(params)?;
        let query_lower = params.query.to_lowercase();

        let mut results = Vec::new();

        // Search all .md files in memory directory
        for entry in fs::read_dir(&self.memory_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Ok(content) = fs::read_to_string(&path) {
                    // Simple case-insensitive search
                    if content.to_lowercase().contains(&query_lower) {
                        let filename = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");

                        // Extract relevant context (paragraph containing query)
                        let context = extract_context(&content, &query_lower);

                        results.push(format!("**{}**\n{}", filename, context));
                    }
                }
            }
        }

        if results.is_empty() {
            Ok(format!("No memories found for: {}", params.query))
        } else {
            results.truncate(params.limit);
            Ok(format!("Found {} relevant memories:\n\n{}",
                results.len(),
                results.join("\n\n---\n\n")
            ))
        }
    }
}

fn extract_context(content: &str, query: &str) -> String {
    // Find paragraph containing query
    for paragraph in content.split("\n\n") {
        if paragraph.to_lowercase().contains(query) {
            // Return up to 300 chars of context
            let trimmed = paragraph.chars().take(300).collect::<String>();
            return if paragraph.len() > 300 {
                format!("{}...", trimmed)
            } else {
                trimmed
            };
        }
    }

    // Fallback: first 200 chars
    content.chars().take(200).collect::<String>() + "..."
}
```

**Files to Modify:**
- `src/tools/mod.rs` - Register the tool

**Testing:**
```bash
# 1. Create test memory file
mkdir -p soul/data/memory
echo "## Scheduler Issue

When scheduler doesn't run, check:
1. Database permissions
2. Service status
3. Restart sandy.service

Solution: sudo systemctl restart sandy" > soul/data/memory/solutions.md

# 2. Rebuild
cargo build --release && sudo systemctl restart sandy

# 3. Test via Telegram
"Search memory for scheduler issue"

Expected output:
"Found 1 relevant memory:
**solutions.md**
When scheduler doesn't run, check:
1. Database permissions..."
```

**Success Criteria:**
- [ ] Tool registered and shows in `list_tools`
- [ ] Search finds content in memory files
- [ ] Returns relevant context (not entire file)
- [ ] Returns empty message if nothing found

---

### Task 3: Add Memory Write Tool
**Time:** 1.5 hours
**Value:** High - Enables learning persistence
**Dependencies:** None

**Objective:**
Let Sandy write to memory files to record learnings.

**Files to Create:**
- `src/tools/memory_write.rs`

**Implementation:**
```rust
use crate::tools::Tool;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub struct MemoryWriteTool {
    memory_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct MemoryWriteParams {
    category: String,  // "solutions", "errors", "patterns", "insights"
    content: String,
}

impl MemoryWriteTool {
    pub fn new(memory_dir: PathBuf) -> Self {
        Self { memory_dir }
    }
}

#[async_trait::async_trait]
impl Tool for MemoryWriteTool {
    fn name(&self) -> &str { "write_memory" }

    fn description(&self) -> &str {
        "Write to long-term memory. Use this to record solutions, patterns, errors, or insights that should persist across sessions."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "enum": ["solutions", "errors", "patterns", "insights"],
                    "description": "What type of memory to record"
                },
                "content": {
                    "type": "string",
                    "description": "What to remember (be specific and include context)"
                }
            },
            "required": ["category", "content"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let params: MemoryWriteParams = serde_json::from_value(params)?;

        // Create memory directory if it doesn't exist
        std::fs::create_dir_all(&self.memory_dir)?;

        let file_path = self.memory_dir.join(format!("{}.md", params.category));

        // Format entry with timestamp
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let entry = format!("\n## {}\n\n{}\n", timestamp, params.content);

        // Append to file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        file.write_all(entry.as_bytes())?;

        Ok(format!("✅ Recorded to {}.md", params.category))
    }
}
```

**Files to Modify:**
- `src/tools/mod.rs` - Register the tool

**Testing:**
```bash
# Rebuild
cargo build --release && sudo systemctl restart sandy

# Test via Telegram:
"Write to memory: category=solutions, content='When reminders fail, check tool name matches in AGENTS.md'"

# Verify file was created:
cat soul/data/memory/solutions.md

Expected:
## 2026-02-16 22:30:00 UTC

When reminders fail, check tool name matches in AGENTS.md
```

**Success Criteria:**
- [ ] Tool creates memory directory if missing
- [ ] Appends entries with timestamps
- [ ] Supports all categories (solutions, errors, patterns, insights)
- [ ] Multiple writes to same file append (don't overwrite)

---

### Task 4: Update AGENTS.md with Memory Tools
**Time:** 20 minutes
**Value:** Medium - Teaches Sandy to use memory
**Dependencies:** Tasks 2, 3

**Objective:**
Document memory tools so Sandy knows when to use them.

**Files to Modify:**
- `soul/AGENTS.md`

**Changes:**
Add new section after "Agent Orchestration":
```markdown
## Memory & Learning Tools

Use these tools to remember solutions and learn from mistakes.

### search_memory
Search past memories before attempting solutions.

**When to use:**
- Before fixing an error: "Have I seen this error before?"
- Before creating a solution: "Did I solve this already?"
- When user asks a repeated question

**Parameters:**
- `query` (required): What to search for
- `limit` (optional): Max results (default: 5)

**Example:**
User: "The scheduler isn't working"
You: [First, search memory]
```json
{
  "tool": "search_memory",
  "params": {
    "query": "scheduler not working"
  }
}
```
[If found, apply previous solution. If not, troubleshoot and record solution]

### write_memory
Record learnings that should persist across sessions.

**When to use:**
- After fixing a complex problem → record solution
- When user teaches you something → record insight
- When you discover a pattern → record pattern
- When an error keeps happening → record error analysis

**Parameters:**
- `category` (required): "solutions", "errors", "patterns", or "insights"
- `content` (required): What to remember

**Example:**
After successfully fixing scheduler:
```json
{
  "tool": "write_memory",
  "params": {
    "category": "solutions",
    "content": "Scheduler fix: When reminders don't fire, check tool names in AGENTS.md match actual implementations. Run doctor command to verify."
  }
}
```

### Memory Workflow Pattern

**Standard workflow for problem-solving:**
1. User reports problem
2. **Search memory first**: `search_memory(query="problem keywords")`
3. If found: Apply previous solution
4. If not found: Troubleshoot normally
5. **Record solution**: `write_memory(category="solutions", content="what worked")`

This way, you learn and improve over time.
```

**Testing:**
```bash
# Rebuild
cargo build --release && sudo systemctl restart sandy

# Test via Telegram:
"Help! The scheduler stopped working again"

Expected behavior:
1. Sandy searches memory: search_memory("scheduler")
2. Finds previous solution
3. Applies it
4. Reports: "I found a solution in my memory from last time..."
```

**Success Criteria:**
- [ ] Documentation is clear and includes examples
- [ ] Sandy uses search_memory before problem-solving
- [ ] Sandy writes to memory after solving issues

---

## Priority 2: Error Recovery & Resilience

These make Sandy more robust and self-healing.

---

### Task 5: Add Backoff Policy Module
**Time:** 1 hour
**Value:** Medium - Foundation for retry logic
**Dependencies:** None

**Objective:**
Create reusable exponential backoff utility.

**Files to Create:**
- `src/backoff.rs`

**Implementation:**
```rust
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct BackoffPolicy {
    pub initial_ms: u64,
    pub max_ms: u64,
    pub factor: f64,
    pub jitter: f64,
}

impl BackoffPolicy {
    pub fn default_network() -> Self {
        Self {
            initial_ms: 1000,   // 1 second
            max_ms: 60000,      // 1 minute
            factor: 2.0,        // Double each time
            jitter: 0.1,        // 10% randomness
        }
    }

    pub fn rate_limit() -> Self {
        Self {
            initial_ms: 5000,   // 5 seconds
            max_ms: 300000,     // 5 minutes
            factor: 2.0,
            jitter: 0.2,        // 20% randomness
        }
    }

    pub fn compute(&self, attempt: u32) -> Duration {
        use rand::Rng;

        let base = self.initial_ms as f64
            * self.factor.powi(attempt.saturating_sub(1) as i32);

        let jitter = base * self.jitter * rand::thread_rng().gen::<f64>();
        let total_ms = (base + jitter).min(self.max_ms as f64);

        Duration::from_millis(total_ms as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_increases() {
        let policy = BackoffPolicy::default_network();

        let delay1 = policy.compute(1);
        let delay2 = policy.compute(2);
        let delay3 = policy.compute(3);

        // Each delay should be roughly double (accounting for jitter)
        assert!(delay2 > delay1);
        assert!(delay3 > delay2);
    }

    #[test]
    fn test_backoff_caps_at_max() {
        let policy = BackoffPolicy {
            initial_ms: 1000,
            max_ms: 5000,
            factor: 2.0,
            jitter: 0.0, // No jitter for predictable test
        };

        // After many attempts, should cap at max_ms
        let delay = policy.compute(10);
        assert!(delay <= Duration::from_millis(5000));
    }
}
```

**Files to Modify:**
- `src/lib.rs` or `src/main.rs` - Add `mod backoff;`

**Testing:**
```bash
cargo test backoff

Expected:
test backoff::tests::test_backoff_increases ... ok
test backoff::tests::test_backoff_caps_at_max ... ok
```

**Success Criteria:**
- [ ] Module compiles
- [ ] Tests pass
- [ ] Backoff increases exponentially
- [ ] Backoff caps at max_ms

---

### Task 6: Add Error Classifier
**Time:** 1 hour
**Value:** Medium - Foundation for smart retries
**Dependencies:** None

**Objective:**
Classify errors as recoverable/permanent/rate-limit.

**Files to Create:**
- `src/error_classifier.rs`

**Implementation:**
```rust
use std::io;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorClass {
    Recoverable,    // Retry with normal backoff
    RateLimit,      // Retry with longer backoff
    Permanent,      // Don't retry
    Auth,           // Refresh token, then retry
}

pub fn classify_llm_error(error: &reqwest::Error) -> ErrorClass {
    // Check status code
    if let Some(status) = error.status() {
        match status.as_u16() {
            429 => return ErrorClass::RateLimit,
            401 | 403 => return ErrorClass::Auth,
            400 | 404 | 422 => return ErrorClass::Permanent, // Client errors
            500..=599 => return ErrorClass::Recoverable,      // Server errors
            _ => {}
        }
    }

    // Check if it's a network/timeout error
    if error.is_timeout() || error.is_connect() {
        return ErrorClass::Recoverable;
    }

    // Default to recoverable for safety
    ErrorClass::Recoverable
}

pub fn classify_io_error(error: &io::Error) -> ErrorClass {
    use io::ErrorKind::*;

    match error.kind() {
        TimedOut | ConnectionReset | ConnectionAborted | ConnectionRefused => {
            ErrorClass::Recoverable
        }
        PermissionDenied => ErrorClass::Auth,
        NotFound | InvalidInput | InvalidData => ErrorClass::Permanent,
        _ => ErrorClass::Recoverable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_timeout() {
        let err = io::Error::new(io::ErrorKind::TimedOut, "timeout");
        assert_eq!(classify_io_error(&err), ErrorClass::Recoverable);
    }

    #[test]
    fn test_classify_permission() {
        let err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        assert_eq!(classify_io_error(&err), ErrorClass::Auth);
    }
}
```

**Files to Modify:**
- Add `mod error_classifier;` to lib/main

**Testing:**
```bash
cargo test error_classifier

Expected:
test error_classifier::tests::test_classify_timeout ... ok
test error_classifier::tests::test_classify_permission ... ok
```

**Success Criteria:**
- [ ] Correctly classifies rate limits (429)
- [ ] Correctly classifies auth errors (401, 403)
- [ ] Correctly classifies network errors
- [ ] Tests pass

---

### Task 7: Wrap LLM Calls with Retry Logic
**Time:** 2 hours
**Value:** High - Makes Sandy resilient to API failures
**Dependencies:** Tasks 5, 6

**Objective:**
Add automatic retry with backoff to all LLM API calls.

**Files to Modify:**
- `src/llm.rs`

**Changes:**
```rust
use crate::backoff::{BackoffPolicy};
use crate::error_classifier::{classify_llm_error, ErrorClass};

impl LLMClient {
    // Replace existing call method with this:
    pub async fn call(&self, messages: Vec<Message>) -> Result<String, Error> {
        self.call_with_retry(messages, 5).await
    }

    async fn call_with_retry(
        &self,
        messages: Vec<Message>,
        max_attempts: u32,
    ) -> Result<String, Error> {
        let policy = BackoffPolicy::default_network();
        let mut attempt = 0;

        loop {
            attempt += 1;

            log::debug!("LLM call attempt {}/{}", attempt, max_attempts);

            match self.raw_call(messages.clone()).await {
                Ok(response) => {
                    log::debug!("LLM call succeeded on attempt {}", attempt);
                    return Ok(response);
                }
                Err(error) => {
                    let error_class = if let Error::Api(ref e) = error {
                        classify_llm_error(e)
                    } else {
                        ErrorClass::Permanent
                    };

                    match error_class {
                        ErrorClass::Permanent => {
                            log::error!("Permanent error, not retrying: {}", error);
                            return Err(error);
                        }
                        ErrorClass::Auth => {
                            // TODO: Implement token refresh in future task
                            log::error!("Auth error, not retrying yet: {}", error);
                            return Err(error);
                        }
                        ErrorClass::Recoverable | ErrorClass::RateLimit => {
                            if attempt >= max_attempts {
                                log::error!("Max attempts reached, giving up: {}", error);
                                return Err(error);
                            }

                            let wait = policy.compute(attempt);
                            log::warn!(
                                "Attempt {} failed ({}), retrying in {:?}",
                                attempt,
                                error_class,
                                wait
                            );

                            tokio::time::sleep(wait).await;
                        }
                    }
                }
            }
        }
    }

    // Rename current call() to raw_call()
    async fn raw_call(&self, messages: Vec<Message>) -> Result<String, Error> {
        // Existing implementation...
    }
}
```

**Testing:**
```bash
# Rebuild
cargo build --release

# Test by simulating API failure:
# 1. Temporarily set invalid API key in config
# 2. Try using Sandy
# 3. Check logs for retry attempts

# Check logs:
tail -f logs/sandy.log | grep -i "attempt\|retry"

Expected:
[WARN] Attempt 1 failed (Recoverable), retrying in 1.2s
[WARN] Attempt 2 failed (Recoverable), retrying in 2.4s
[WARN] Attempt 3 failed (Recoverable), retrying in 5.1s
```

**Success Criteria:**
- [ ] LLM calls retry on network errors
- [ ] Backoff increases between attempts
- [ ] Permanent errors don't retry
- [ ] Max attempts respected
- [ ] Logs show retry attempts

---

## Priority 3: Self-Diagnosis (Doctor Command)

These enable Sandy to detect and fix her own issues.

---

### Task 8: Create Doctor Command Structure
**Time:** 1 hour
**Value:** Medium - Foundation for self-healing
**Dependencies:** None

**Objective:**
Add `sandy doctor` CLI command with basic structure.

**Files to Create:**
- `src/doctor/mod.rs`

**Implementation:**
```rust
use colored::Colorize;

pub struct HealthCheck {
    pub name: String,
    pub severity: Severity,
    pub status: CheckStatus,
    pub message: String,
    pub fix: Option<Fix>,
}

pub enum Severity {
    Critical,   // Blocks core functionality
    Warning,    // Degrades functionality
    Info,       // Optimization opportunity
}

pub enum CheckStatus {
    Pass,
    Fail,
    Warning,
}

pub struct Fix {
    pub description: String,
    pub auto_fixable: bool,
    pub fix_fn: Box<dyn Fn() -> Result<(), String>>,
}

pub struct DoctorOptions {
    pub repair: bool,
    pub force: bool,
}

pub async fn run_doctor(options: DoctorOptions) -> Result<(), String> {
    println!("{}", "🏥 Sandy Health Check".bold());
    println!();

    let checks = run_all_checks().await;

    // Print results
    for check in &checks {
        let status_icon = match check.status {
            CheckStatus::Pass => "✅",
            CheckStatus::Warning => "⚠️ ",
            CheckStatus::Fail => "❌",
        };

        println!("{} {}: {}", status_icon, check.name, check.message);
    }

    // Summary
    let critical_failures: Vec<_> = checks.iter()
        .filter(|c| c.status == CheckStatus::Fail && matches!(c.severity, Severity::Critical))
        .collect();

    if !critical_failures.is_empty() {
        println!();
        println!("{}", format!("❌ {} critical issues found", critical_failures.len()).red());

        if options.repair {
            println!();
            println!("🔧 Attempting repairs...");
            auto_repair(&checks, options.force).await?;
        } else {
            println!("Run with --repair to fix automatically");
        }
    } else {
        println!();
        println!("{}", "✅ All critical checks passed!".green());
    }

    Ok(())
}

async fn run_all_checks() -> Vec<HealthCheck> {
    vec![
        // Will add actual checks in next tasks
        HealthCheck {
            name: "Configuration".to_string(),
            severity: Severity::Critical,
            status: CheckStatus::Pass,
            message: "Config file is valid".to_string(),
            fix: None,
        }
    ]
}

async fn auto_repair(checks: &[HealthCheck], force: bool) -> Result<(), String> {
    for check in checks {
        if check.status == CheckStatus::Fail {
            if let Some(fix) = &check.fix {
                if fix.auto_fixable || force {
                    println!("  Fixing: {}", check.name);
                    (fix.fix_fn)()?;
                    println!("  ✅ Fixed");
                }
            }
        }
    }
    Ok(())
}
```

**Files to Modify:**
- `src/main.rs` - Add doctor command

```rust
mod doctor;

enum Command {
    Start,
    Stop,
    Doctor { repair: bool, force: bool },
}

// In main():
Command::Doctor { repair, force } => {
    doctor::run_doctor(doctor::DoctorOptions { repair, force }).await?;
}
```

**Testing:**
```bash
cargo build --release
./target/release/sandy doctor

Expected output:
🏥 Sandy Health Check

✅ Configuration: Config file is valid

✅ All critical checks passed!
```

**Success Criteria:**
- [ ] Command compiles and runs
- [ ] Shows health check results
- [ ] Color-coded output
- [ ] --repair flag accepted (but doesn't do anything yet)

---

### Task 9: Add Tool Registry Check
**Time:** 1 hour
**Value:** High - Catches tool mismatches
**Dependencies:** Task 8

**Objective:**
Detect mismatches between AGENTS.md and registered tools (like we had with schedule tools).

**Files to Modify:**
- `src/doctor/mod.rs`

**Add new check function:**
```rust
use std::collections::HashSet;
use std::fs;

async fn check_tool_registry() -> HealthCheck {
    // 1. Get list of tools documented in AGENTS.md
    let agents_content = match fs::read_to_string("soul/AGENTS.md") {
        Ok(c) => c,
        Err(e) => return HealthCheck {
            name: "Tool Registry".to_string(),
            severity: Severity::Critical,
            status: CheckStatus::Fail,
            message: format!("Cannot read AGENTS.md: {}", e),
            fix: None,
        },
    };

    let documented_tools = extract_tool_names(&agents_content);

    // 2. Get list of registered tools
    let config = match crate::config::load_config() {
        Ok(c) => c,
        Err(e) => return HealthCheck {
            name: "Tool Registry".to_string(),
            severity: Severity::Critical,
            status: CheckStatus::Fail,
            message: format!("Cannot load config: {}", e),
            fix: None,
        },
    };

    let registry = crate::tools::create_tool_registry(&config);
    let registered_tools: HashSet<String> = registry.tool_names().into_iter().collect();

    // 3. Find mismatches
    let missing_in_registry: Vec<_> = documented_tools.iter()
        .filter(|t| !registered_tools.contains(*t))
        .collect();

    let missing_in_docs: Vec<_> = registered_tools.iter()
        .filter(|t| !documented_tools.contains(t))
        .collect();

    if missing_in_registry.is_empty() && missing_in_docs.is_empty() {
        HealthCheck {
            name: "Tool Registry".to_string(),
            severity: Severity::Critical,
            status: CheckStatus::Pass,
            message: format!("{} tools registered and documented", registered_tools.len()),
            fix: None,
        }
    } else {
        let mut message = String::new();
        if !missing_in_registry.is_empty() {
            message.push_str(&format!(
                "Tools documented but not registered: {}. ",
                missing_in_registry.join(", ")
            ));
        }
        if !missing_in_docs.is_empty() {
            message.push_str(&format!(
                "Tools registered but not documented: {}",
                missing_in_docs.join(", ")
            ));
        }

        HealthCheck {
            name: "Tool Registry".to_string(),
            severity: Severity::Critical,
            status: CheckStatus::Fail,
            message,
            fix: Some(Fix {
                description: "Update AGENTS.md or tool registry".to_string(),
                auto_fixable: false,
                fix_fn: Box::new(|| Ok(())),
            }),
        }
    }
}

fn extract_tool_names(content: &str) -> HashSet<String> {
    let mut tools = HashSet::new();

    // Look for tool names in code blocks or after "###"
    for line in content.lines() {
        if line.starts_with("### ") {
            let tool_name = line.trim_start_matches("### ").trim();
            if !tool_name.is_empty() && tool_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                tools.insert(tool_name.to_string());
            }
        }
    }

    tools
}

// Update run_all_checks():
async fn run_all_checks() -> Vec<HealthCheck> {
    vec![
        check_config().await,
        check_tool_registry().await,
    ]
}
```

**Testing:**
```bash
cargo build --release

# Test 1: Everything matches
./target/release/sandy doctor
Expected: ✅ Tool Registry: 25 tools registered and documented

# Test 2: Introduce mismatch
# Edit AGENTS.md and add "### fake_tool"
./target/release/sandy doctor
Expected: ❌ Tool Registry: Tools documented but not registered: fake_tool
```

**Success Criteria:**
- [ ] Detects tools in AGENTS.md but not registered
- [ ] Detects registered tools not in AGENTS.md
- [ ] Passes when everything matches
- [ ] Shows clear error messages

---

### Task 10: Add Database Integrity Check
**Time:** 1.5 hours
**Value:** Medium - Prevents data corruption
**Dependencies:** Task 8

**Objective:**
Verify database exists, is accessible, and has correct schema.

**Files to Modify:**
- `src/doctor/mod.rs`

**Add check function:**
```rust
async fn check_database() -> HealthCheck {
    let db_path = std::path::Path::new("soul/data/runtime/microclaw.db");

    // Check if database file exists
    if !db_path.exists() {
        return HealthCheck {
            name: "Database".to_string(),
            severity: Severity::Critical,
            status: CheckStatus::Fail,
            message: "Database file not found".to_string(),
            fix: Some(Fix {
                description: "Create database with schema".to_string(),
                auto_fixable: true,
                fix_fn: Box::new(|| {
                    // Initialize database
                    crate::database::initialize_database()?;
                    Ok(())
                }),
            }),
        };
    }

    // Try to open and query
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => return HealthCheck {
            name: "Database".to_string(),
            severity: Severity::Critical,
            status: CheckStatus::Fail,
            message: format!("Cannot open database: {}", e),
            fix: None,
        },
    };

    // Check for required tables
    let required_tables = vec![
        "scheduled_tasks",
        "task_run_logs",
        "chats",
        "messages",
        "sessions",
    ];

    for table in &required_tables {
        let query = format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{}';", table);
        let exists: Result<String, _> = conn.query_row(&query, [], |row| row.get(0));

        if exists.is_err() {
            return HealthCheck {
                name: "Database".to_string(),
                severity: Severity::Critical,
                status: CheckStatus::Fail,
                message: format!("Missing table: {}", table),
                fix: Some(Fix {
                    description: "Run database migrations".to_string(),
                    auto_fixable: true,
                    fix_fn: Box::new(|| {
                        crate::database::run_migrations()?;
                        Ok(())
                    }),
                }),
            };
        }
    }

    HealthCheck {
        name: "Database".to_string(),
        severity: Severity::Critical,
        status: CheckStatus::Pass,
        message: "Database is healthy".to_string(),
        fix: None,
    }
}
```

**Testing:**
```bash
# Test 1: Normal case
./target/release/sandy doctor
Expected: ✅ Database: Database is healthy

# Test 2: Missing database
mv soul/data/runtime/microclaw.db soul/data/runtime/microclaw.db.backup
./target/release/sandy doctor --repair
Expected:
❌ Database: Database file not found
🔧 Attempting repairs...
  Fixing: Database
  ✅ Fixed

# Restore
mv soul/data/runtime/microclaw.db.backup soul/data/runtime/microclaw.db
```

**Success Criteria:**
- [ ] Detects missing database file
- [ ] Detects missing tables
- [ ] Can auto-create database with --repair
- [ ] Can run migrations with --repair

---

## Priority 4: Autonomous Behavior

These make Sandy more proactive and autonomous.

---

### Task 11: Add "Before Response" Hook System
**Time:** 2 hours
**Value:** High - Enables autonomous actions
**Dependencies:** None

**Objective:**
Create hook system that runs before Sandy generates a response.

**Files to Create:**
- `src/hooks/mod.rs`

**Implementation:**
```rust
use async_trait::async_trait;

pub struct BeforeResponseContext {
    pub user_message: String,
    pub user_id: i64,
    pub chat_id: i64,
}

#[async_trait]
pub trait BeforeResponseHook: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, ctx: &BeforeResponseContext) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;
}

pub struct HookManager {
    hooks: Vec<Box<dyn BeforeResponseHook>>,
}

impl HookManager {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register(&mut self, hook: Box<dyn BeforeResponseHook>) {
        self.hooks.push(hook);
    }

    pub async fn run_before_response(&self, ctx: &BeforeResponseContext) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        for hook in &self.hooks {
            log::debug!("Running hook: {}", hook.name());
            if let Some(action) = hook.execute(ctx).await? {
                log::info!("Hook '{}' triggered action: {}", hook.name(), action);
                return Ok(Some(action));
            }
        }
        Ok(None)
    }
}
```

**Files to Modify:**
- `src/main.rs` - Initialize HookManager
- `src/lib.rs` - Add `pub mod hooks;`

**Testing:**
```bash
cargo test

# Should compile without errors
```

**Success Criteria:**
- [ ] Module compiles
- [ ] HookManager can register hooks
- [ ] run_before_response returns None (no hooks yet)

---

### Task 12: Add Memory Search Hook
**Time:** 1 hour
**Value:** High - Automatic memory lookup
**Dependencies:** Tasks 2, 11

**Objective:**
Before responding to errors, automatically search memory.

**Files to Create:**
- `src/hooks/memory_search_hook.rs`

**Implementation:**
```rust
use crate::hooks::{BeforeResponseHook, BeforeResponseContext};
use async_trait::async_trait;

pub struct MemorySearchHook {
    memory_tool: crate::tools::MemorySearchTool,
}

impl MemorySearchHook {
    pub fn new(memory_dir: std::path::PathBuf) -> Self {
        Self {
            memory_tool: crate::tools::MemorySearchTool::new(memory_dir),
        }
    }
}

#[async_trait]
impl BeforeResponseHook for MemorySearchHook {
    fn name(&self) -> &str {
        "memory_search"
    }

    async fn execute(&self, ctx: &BeforeResponseContext) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let msg_lower = ctx.user_message.to_lowercase();

        // Trigger on error-related keywords
        let error_keywords = ["error", "not working", "failed", "broken", "issue", "problem"];
        let has_error = error_keywords.iter().any(|kw| msg_lower.contains(kw));

        if has_error {
            log::info!("Error detected in message, searching memory...");

            // Extract key terms for search
            let search_terms = extract_search_terms(&ctx.user_message);

            if let Some(terms) = search_terms {
                let params = serde_json::json!({
                    "query": terms,
                    "limit": 3
                });

                match self.memory_tool.execute(params).await {
                    Ok(result) if !result.contains("No memories found") => {
                        return Ok(Some(format!(
                            "I found relevant information in my memory:\n\n{}",
                            result
                        )));
                    }
                    _ => {}
                }
            }
        }

        Ok(None)
    }
}

fn extract_search_terms(message: &str) -> Option<String> {
    let msg_lower = message.to_lowercase();

    // Extract key nouns/terms (simple heuristic)
    let words: Vec<&str> = msg_lower.split_whitespace().collect();

    // Look for important words (skip common words)
    let important: Vec<&str> = words.iter()
        .filter(|w| {
            let word = w.to_lowercase();
            !["the", "is", "not", "working", "my", "a", "an", "to", "of"].contains(&word.as_str())
                && word.len() > 3
        })
        .copied()
        .collect();

    if important.is_empty() {
        None
    } else {
        Some(important.join(" "))
    }
}
```

**Files to Modify:**
- `src/hooks/mod.rs` - Add `pub mod memory_search_hook;`
- `src/main.rs` - Register the hook

```rust
let mut hook_manager = hooks::HookManager::new();
hook_manager.register(Box::new(
    hooks::memory_search_hook::MemorySearchHook::new(
        PathBuf::from("soul/data/memory")
    )
));
```

**Files to Modify (message handler):**
- Where you handle user messages, add:

```rust
// Before calling LLM:
if let Some(action) = hook_manager.run_before_response(&ctx).await? {
    // Memory found something, prepend to context
    system_prompt.push_str(&format!("\n\n{}", action));
}
```

**Testing:**
```bash
# 1. Create test memory
echo "## Scheduler Issue
When scheduler doesn't run:
1. Check service status
2. Verify database permissions" > soul/data/memory/solutions.md

# 2. Rebuild and restart
cargo build --release && sudo systemctl restart sandy

# 3. Test via Telegram
"Help, the scheduler is not working"

Expected:
Sandy's response includes: "I found relevant information in my memory: ..."
```

**Success Criteria:**
- [ ] Hook triggers on error keywords
- [ ] Searches memory automatically
- [ ] Prepends memory results to context
- [ ] Works silently (no user-visible delay)

---

## Priority 5: Dynamic Skill Creation

Enable Sandy to create her own tools.

---

### Task 13: Add Skill Creator Tool (Basic)
**Time:** 2 hours
**Value:** High - Enables autonomous expansion
**Dependencies:** None

**Objective:**
Let Sandy create simple skill files.

**Files to Create:**
- `src/tools/create_skill.rs`

**Implementation:**
```rust
use crate::tools::Tool;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub struct CreateSkillTool {
    skills_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct CreateSkillParams {
    name: String,
    description: String,
    instructions: String,
}

impl CreateSkillTool {
    pub fn new(skills_dir: PathBuf) -> Self {
        Self { skills_dir }
    }
}

#[async_trait::async_trait]
impl Tool for CreateSkillTool {
    fn name(&self) -> &str { "create_skill" }

    fn description(&self) -> &str {
        "Create a new skill that teaches you how to handle specific tasks. Use this when you encounter a task you don't have tools for."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Skill name (lowercase, underscores, no spaces)"
                },
                "description": {
                    "type": "string",
                    "description": "What this skill does"
                },
                "instructions": {
                    "type": "string",
                    "description": "Step-by-step instructions for how to perform the task using existing tools"
                }
            },
            "required": ["name", "description", "instructions"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let params: CreateSkillParams = serde_json::from_value(params)?;

        // Validate name
        if !params.name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Skill name must be alphanumeric with underscores only".into());
        }

        // Create skills directory if needed
        fs::create_dir_all(&self.skills_dir)?;

        // Create skill directory
        let skill_dir = self.skills_dir.join(&params.name);
        if skill_dir.exists() {
            return Err(format!("Skill '{}' already exists", params.name).into());
        }
        fs::create_dir_all(&skill_dir)?;

        // Generate SKILL.md
        let skill_content = format!(
r#"# {}

{}

## Instructions

{}
"#,
            params.name,
            params.description,
            params.instructions
        );

        fs::write(skill_dir.join("SKILL.md"), skill_content)?;

        Ok(format!(
            "✅ Skill '{}' created at {}\n\nTo use it, reference these instructions when handling similar tasks.",
            params.name,
            skill_dir.display()
        ))
    }
}
```

**Files to Modify:**
- `src/tools/mod.rs` - Register tool

**Testing:**
```bash
cargo build --release && sudo systemctl restart sandy

# Test via Telegram:
"Create a skill: name=weather, description='Get weather info', instructions='1. Use bash tool to run: curl wttr.in/CITY?format=3\n2. Parse output\n3. Respond with weather'"

# Verify file created:
cat skills/weather/SKILL.md

Expected:
# weather

Get weather info

## Instructions

1. Use bash tool to run: curl wttr.in/CITY?format=3
2. Parse output
3. Respond with weather
```

**Success Criteria:**
- [ ] Tool creates skill directory
- [ ] SKILL.md has correct format
- [ ] Validates skill name
- [ ] Prevents duplicate skill names

---

### Task 14: Load Skills into System Prompt
**Time:** 1.5 hours
**Value:** High - Makes skills usable
**Dependencies:** Task 13

**Objective:**
Automatically load skill instructions into Sandy's system prompt.

**Files to Create:**
- `src/skills/mod.rs`

**Implementation:**
```rust
use std::fs;
use std::path::{Path, PathBuf};

pub struct SkillRegistry {
    skills_dir: PathBuf,
}

impl SkillRegistry {
    pub fn new(skills_dir: PathBuf) -> Self {
        Self { skills_dir }
    }

    pub fn load_all_skills(&self) -> Result<String, Box<dyn std::error::Error>> {
        if !self.skills_dir.exists() {
            return Ok(String::new());
        }

        let mut skills_prompt = String::from("\n## Available Skills\n\n");
        skills_prompt.push_str("You have learned these skills from past experiences:\n\n");

        for entry in fs::read_dir(&self.skills_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let skill_file = entry.path().join("SKILL.md");
                if skill_file.exists() {
                    let content = fs::read_to_string(&skill_file)?;
                    skills_prompt.push_str(&format!("---\n\n{}\n", content));
                }
            }
        }

        Ok(skills_prompt)
    }

    pub fn count_skills(&self) -> usize {
        if !self.skills_dir.exists() {
            return 0;
        }

        fs::read_dir(&self.skills_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.file_type().ok().map_or(false, |t| t.is_dir()))
                    .filter(|e| e.path().join("SKILL.md").exists())
                    .count()
            })
            .unwrap_or(0)
    }
}
```

**Files to Modify:**
- `src/lib.rs` - Add `pub mod skills;`
- `src/llm.rs` or wherever system prompt is built:

```rust
use crate::skills::SkillRegistry;

// When building system prompt:
let skill_registry = SkillRegistry::new(PathBuf::from("skills"));
let skills_content = skill_registry.load_all_skills()?;

system_prompt.push_str(&skills_content);
```

**Testing:**
```bash
# After creating weather skill in Task 13:
cargo build --release && sudo systemctl restart sandy

# Test via Telegram:
"What's the weather in Stockholm?"

Expected:
Sandy uses the weather skill instructions automatically
(Uses bash tool with curl wttr.in/Stockholm)
```

**Success Criteria:**
- [ ] Skills loaded at startup
- [ ] Count shows correct number
- [ ] Skills appear in system prompt
- [ ] Sandy uses skill instructions automatically

---

## Priority 6: Advanced Features

Optional enhancements for even more autonomy.

---

### Task 15: Add Auto-Heal on Startup
**Time:** 1 hour
**Value:** Medium - Proactive problem fixing
**Dependencies:** Tasks 8-10

**Objective:**
Run basic health checks on startup and auto-fix critical issues.

**Files to Modify:**
- `src/main.rs`

**Changes:**
```rust
async fn start_sandy(config: Config) -> Result<(), Error> {
    log::info!("Running startup health checks...");

    // Run non-critical checks (don't block startup)
    let checks = doctor::run_all_checks().await;

    let critical_failures: Vec<_> = checks.iter()
        .filter(|c| c.status == CheckStatus::Fail && matches!(c.severity, Severity::Critical))
        .collect();

    if !critical_failures.is_empty() {
        log::warn!("⚠️  {} critical issues detected", critical_failures.len());

        for check in &critical_failures {
            log::error!("  - {}: {}", check.name, check.message);
        }

        // Try auto-repair
        log::info!("Attempting auto-repair...");
        match doctor::auto_repair(&checks, false).await {
            Ok(()) => log::info!("✅ Auto-repair completed"),
            Err(e) => {
                log::error!("❌ Auto-repair failed: {}", e);
                return Err(format!("Critical issues detected. Run 'sandy doctor --repair' manually").into());
            }
        }
    } else {
        log::info!("✅ All health checks passed");
    }

    // Continue with normal startup...
    Ok(())
}
```

**Testing:**
```bash
# Test 1: Normal startup
cargo build --release && sudo systemctl restart sandy
tail -f logs/sandy.log

Expected:
[INFO] Running startup health checks...
[INFO] ✅ All health checks passed

# Test 2: Broken database
mv soul/data/runtime/microclaw.db soul/data/runtime/microclaw.db.backup
sudo systemctl restart sandy

Expected logs:
[WARN] ⚠️  1 critical issues detected
[ERROR]   - Database: Database file not found
[INFO] Attempting auto-repair...
[INFO] ✅ Auto-repair completed

# Verify database recreated:
ls soul/data/runtime/microclaw.db  # Should exist
```

**Success Criteria:**
- [ ] Health checks run on startup
- [ ] Critical failures logged
- [ ] Auto-repair attempts fixes
- [ ] Startup continues if repair succeeds
- [ ] Startup fails if critical issues remain

---

### Task 16: Add Session End Memory Flush
**Time:** 1 hour
**Value:** Low - Nice to have
**Dependencies:** Task 3

**Objective:**
Automatically record session summary when conversation ends.

**Files to Create:**
- `src/hooks/session_end.rs`

**Implementation:**
```rust
pub struct SessionEndHook {
    memory_tool: crate::tools::MemoryWriteTool,
}

impl SessionEndHook {
    pub fn new(memory_dir: PathBuf) -> Self {
        Self {
            memory_tool: crate::tools::MemoryWriteTool::new(memory_dir),
        }
    }

    pub async fn on_session_end(&self, chat_id: i64, message_count: usize) {
        // Write session stats to memory
        let content = format!(
            "Session ended\nChat: {}\nMessages: {}",
            chat_id, message_count
        );

        let params = serde_json::json!({
            "category": "insights",
            "content": content
        });

        let _ = self.memory_tool.execute(params).await;
    }
}
```

**Testing:**
Manual verification - check `soul/data/memory/insights.md` after conversation ends.

**Success Criteria:**
- [ ] Session stats recorded
- [ ] No errors during write
- [ ] File grows over time

---

## Summary: Implementation Order

### Quick Wins (Start Here) - 1-2 days
1. Task 1: Autonomous task recognition keywords
2. Task 2: Memory search tool
3. Task 3: Memory write tool
4. Task 4: Document memory tools

**Impact:** Sandy becomes noticeably smarter - uses agents autonomously, remembers solutions

### Resilience Layer - 2-3 days
5. Task 5: Backoff policy
6. Task 6: Error classifier
7. Task 7: LLM retry logic

**Impact:** Sandy handles API failures gracefully

### Self-Diagnosis - 2-3 days
8. Task 8: Doctor command structure
9. Task 9: Tool registry check
10. Task 10: Database integrity check

**Impact:** Sandy can diagnose and fix her own issues

### Autonomous Behavior - 2-3 days
11. Task 11: Hook system
12. Task 12: Memory search hook

**Impact:** Sandy proactively searches memory before responding

### Skill Creation - 2-3 days
13. Task 13: Create skill tool
14. Task 14: Load skills at startup

**Impact:** Sandy can expand her own capabilities

### Polish - 1-2 days
15. Task 15: Auto-heal on startup
16. Task 16: Session memory flush

**Impact:** Smoother operation, better learning

---

**Total Estimated Time:** 2-3 weeks for all tasks
**First Milestone (Tasks 1-4):** 1-2 days → Immediate user-visible improvements

Each task is atomic, testable, and provides value on its own!
