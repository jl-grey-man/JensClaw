# Sandy Self-Healing & Autonomous Evolution Implementation Plan
**Date:** 2026-02-16
**Inspired by:** OpenClaw architecture analysis

---

## Vision

Transform Sandy from a **reactive assistant** into an **autonomous, self-healing agent** that:
- Recognizes task complexity and spawns specialized agents
- Detects and fixes problems automatically
- Creates new tools/skills when needed
- Learns from mistakes and improves over time
- Evolves her capabilities without human intervention

---

## Phase 1: Self-Aware Task Assessment 🧠

### Goal
Enable Sandy to autonomously decide when to delegate work to specialized agents.

### Implementation

#### 1.1 Task Complexity Analyzer
**File:** `src/task_analyzer.rs`

```rust
pub struct TaskAnalysis {
    complexity: Complexity,
    recommended_agent: Option<String>,
    reasoning: String,
    should_delegate: bool,
}

pub enum Complexity {
    Simple,        // Sandy can handle directly
    Medium,        // Could delegate for efficiency
    Complex,       // Should definitely delegate
    Parallel,      // Spawn multiple agents
}

impl TaskAnalyzer {
    pub fn analyze(&self, user_message: &str, context: &Context) -> TaskAnalysis {
        // Analyze message for keywords and patterns:
        // - "research" → spawn Zilla
        // - "write article/summary" → spawn Gonza
        // - "research X and write Y" → execute_workflow
        // - "analyze large file" → spawn research agent
        // - Multiple unrelated tasks → spawn parallel agents
    }
}
```

#### 1.2 Update SOUL.md with Self-Assessment Instructions
**File:** `soul/SOUL.md`

Add section:
```markdown
## Autonomous Task Assessment

Before responding to ANY request, evaluate:

1. **Can I do this directly?**
   - Simple conversation → respond directly
   - Basic tool use → use tools directly

2. **Should I delegate?**
   - Research needed → spawn Zilla
   - Writing needed → spawn Gonza
   - Both → use execute_workflow
   - File too large (>500 lines) → spawn analysis agent
   - Multiple unrelated tasks → spawn parallel agents

3. **Do I have the tools?**
   - Missing tool → check if I can create a skill
   - API not available → search for alternative
   - Repeated task → offer to create automation

**Key Principle:** Be proactive about delegation. Don't wait for explicit "use Zilla" commands.

Example:
User: "What's happening in AI this week?"
Sandy thinking: This requires web research → spawn Zilla automatically
Sandy response: "Let me research that for you..." [spawns Zilla]
```

#### 1.3 Pre-Response Hook
**File:** `src/llm.rs`

```rust
// Before sending user message to LLM:
fn pre_response_hook(&self, message: &str) -> Option<AutoAction> {
    let analysis = self.task_analyzer.analyze(message, &self.context);

    if analysis.should_delegate {
        return Some(AutoAction::SpawnAgent {
            agent: analysis.recommended_agent.unwrap(),
            reasoning: analysis.reasoning,
        });
    }

    None
}
```

---

## Phase 2: Doctor Command (Self-Healing) 🔧

### Goal
Implement `sandy doctor` command that detects and fixes common issues.

### Implementation

#### 2.1 Health Check System
**File:** `src/doctor/mod.rs`

```rust
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
```

#### 2.2 Checks to Implement
**File:** `src/doctor/checks.rs`

```rust
pub fn run_all_checks(config: &Config) -> Vec<HealthCheck> {
    vec![
        check_api_keys(config),
        check_database_integrity(),
        check_tool_registry(),
        check_agent_configs(),
        check_telegram_connection(config),
        check_disk_space(),
        check_permissions(),
        check_scheduler(),
        check_memory_files(),
        check_orphaned_sessions(),
    ]
}

fn check_api_keys(config: &Config) -> HealthCheck {
    // Verify API keys are set and valid
    // Test connection to LLM provider
    // Check expiration dates
}

fn check_database_integrity() -> HealthCheck {
    // Verify database file exists
    // Check for corruption
    // Validate schema version
    // Offer to run migrations if needed
}

fn check_tool_registry() -> HealthCheck {
    // Verify all documented tools are registered
    // Check for name mismatches (like we had with schedule tools)
    // Validate tool schemas
}

fn check_agent_configs() -> HealthCheck {
    // Verify Zilla, Gonza configs exist
    // Check tool assignments are valid
    // Validate JSON format
}

fn check_orphaned_sessions() -> HealthCheck {
    // Find incomplete agent executions
    // Detect hanging processes
    // Offer to clean up
}
```

#### 2.3 Auto-Repair Logic
**File:** `src/doctor/repair.rs`

```rust
pub fn auto_repair(checks: &[HealthCheck], force: bool) -> RepairResult {
    let mut repaired = vec![];
    let mut failed = vec![];

    for check in checks {
        if check.status == CheckStatus::Fail {
            if let Some(fix) = &check.fix {
                if fix.auto_fixable || force {
                    match (fix.fix_fn)() {
                        Ok(_) => repaired.push(check.name.clone()),
                        Err(e) => failed.push((check.name.clone(), e)),
                    }
                }
            }
        }
    }

    RepairResult { repaired, failed }
}
```

#### 2.4 CLI Integration
**File:** `src/main.rs`

```rust
pub enum Command {
    Start,
    Stop,
    Doctor {
        repair: bool,
        force: bool,
        non_interactive: bool,
    },
    // ... existing commands
}

// Usage:
// sandy doctor                    # Run health checks
// sandy doctor --repair           # Auto-fix safe issues
// sandy doctor --repair --force   # Fix everything possible
```

#### 2.5 Startup Health Check
**File:** `src/main.rs`

```rust
async fn start_bot(config: Config) {
    // Run basic health checks on startup
    let checks = doctor::run_all_checks(&config);
    let critical_failures: Vec<_> = checks.iter()
        .filter(|c| c.status == CheckStatus::Fail && c.severity == Severity::Critical)
        .collect();

    if !critical_failures.is_empty() {
        eprintln!("⚠️  Critical issues detected:");
        for check in critical_failures {
            eprintln!("  - {}: {}", check.name, check.message);
        }
        eprintln!("\nRun `sandy doctor --repair` to fix automatically.");
        std::process::exit(1);
    }

    // Continue with startup...
}
```

---

## Phase 3: Error Recovery & Backoff ⏱️

### Goal
Implement intelligent error recovery with exponential backoff and circuit breakers.

### Implementation

#### 3.1 Backoff Strategy
**File:** `src/backoff.rs`

```rust
pub struct BackoffPolicy {
    pub initial_ms: u64,
    pub max_ms: u64,
    pub factor: f64,
    pub jitter: f64,
}

impl BackoffPolicy {
    pub fn compute(&self, attempt: u32) -> Duration {
        let base = self.initial_ms as f64 * self.factor.powi(attempt.saturating_sub(1) as i32);
        let jitter = base * self.jitter * rand::random::<f64>();
        let total_ms = (base + jitter).min(self.max_ms as f64);
        Duration::from_millis(total_ms as u64)
    }
}

// Preset policies:
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
            jitter: 0.2,
        }
    }
}
```

#### 3.2 Error Classification
**File:** `src/error_classifier.rs`

```rust
pub enum ErrorClass {
    Recoverable,    // Retry with backoff
    RateLimit,      // Longer backoff
    Permanent,      // Don't retry
    Auth,           // Refresh token, then retry
}

pub fn classify_error(error: &Error) -> ErrorClass {
    match error {
        Error::Network(e) if is_timeout(e) => ErrorClass::Recoverable,
        Error::Api(status) if status == 429 => ErrorClass::RateLimit,
        Error::Api(status) if status == 401 => ErrorClass::Auth,
        Error::Api(status) if status >= 400 && status < 500 => ErrorClass::Permanent,
        _ => ErrorClass::Recoverable,
    }
}

fn is_timeout(error: &NetworkError) -> bool {
    matches!(error.kind(),
        io::ErrorKind::TimedOut |
        io::ErrorKind::ConnectionReset |
        io::ErrorKind::ConnectionAborted
    )
}
```

#### 3.3 Retry Wrapper
**File:** `src/retry.rs`

```rust
pub async fn retry_with_backoff<F, T, E>(
    operation: F,
    policy: BackoffPolicy,
    max_attempts: u32,
) -> Result<T, E>
where
    F: Fn() -> Future<Output = Result<T, E>>,
    E: std::error::Error,
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                let error_class = classify_error(&error);

                match error_class {
                    ErrorClass::Permanent => return Err(error),
                    ErrorClass::Auth => {
                        // Try to refresh auth
                        if let Err(e) = refresh_auth().await {
                            return Err(e);
                        }
                        continue; // Retry immediately after auth refresh
                    }
                    ErrorClass::Recoverable | ErrorClass::RateLimit => {
                        if attempt >= max_attempts {
                            return Err(error);
                        }

                        let wait = policy.compute(attempt);
                        log::warn!("Attempt {} failed, retrying in {:?}", attempt, wait);
                        tokio::time::sleep(wait).await;
                    }
                }
            }
        }
    }
}
```

#### 3.4 Circuit Breaker
**File:** `src/circuit_breaker.rs`

```rust
pub struct CircuitBreaker {
    state: Arc<Mutex<BreakerState>>,
    config: BreakerConfig,
}

struct BreakerState {
    status: Status,
    failure_count: u32,
    last_failure: Option<Instant>,
    cooldown_until: Option<Instant>,
}

enum Status {
    Closed,      // Normal operation
    Open,        // Failing, reject requests
    HalfOpen,    // Testing if recovered
}

impl CircuitBreaker {
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: Future<Output = Result<T, E>>,
    {
        // Check if circuit is open (too many failures)
        {
            let state = self.state.lock().await;
            if state.status == Status::Open {
                if let Some(cooldown) = state.cooldown_until {
                    if Instant::now() < cooldown {
                        return Err(Error::CircuitOpen);
                    }
                    // Cooldown expired, try half-open
                    state.status = Status::HalfOpen;
                }
            }
        }

        // Execute operation
        match operation.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(error)
            }
        }
    }

    async fn on_failure(&self) {
        let mut state = self.state.lock().await;
        state.failure_count += 1;
        state.last_failure = Some(Instant::now());

        if state.failure_count >= self.config.failure_threshold {
            state.status = Status::Open;
            state.cooldown_until = Some(
                Instant::now() + Duration::from_secs(self.config.cooldown_seconds)
            );
        }
    }

    async fn on_success(&self) {
        let mut state = self.state.lock().await;
        state.failure_count = 0;
        state.status = Status::Closed;
        state.cooldown_until = None;
    }
}
```

#### 3.5 Integration with LLM Calls
**File:** `src/llm.rs`

```rust
impl LLMClient {
    pub async fn call_with_recovery(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, Error> {
        retry_with_backoff(
            || self.raw_call(messages.clone()),
            BackoffPolicy::default_network(),
            5, // Max 5 attempts
        ).await
    }
}
```

---

## Phase 4: Dynamic Skill/Tool Creation 🛠️

### Goal
Enable Sandy to create new tools and skills when she encounters tasks she can't handle.

### Implementation

#### 4.1 Skill Creator Tool
**File:** `src/tools/skill_creator.rs`

```rust
pub struct SkillCreatorTool {
    skills_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct CreateSkillParams {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub required_tools: Vec<String>,
    pub required_env: Vec<String>,
}

impl Tool for SkillCreatorTool {
    async fn execute(&self, params: CreateSkillParams) -> Result<String, Error> {
        // 1. Create skill directory
        let skill_dir = self.skills_dir.join(&params.name);
        fs::create_dir_all(&skill_dir)?;

        // 2. Generate SKILL.md
        let skill_content = format!(
            r#"---
name: {}
description: "{}"
metadata:
  openclaw:
    requires:
      tools: {}
      env: {}
---

# {}

{}
"#,
            params.name,
            params.description,
            serde_json::to_string(&params.required_tools)?,
            serde_json::to_string(&params.required_env)?,
            params.name,
            params.instructions
        );

        fs::write(skill_dir.join("SKILL.md"), skill_content)?;

        // 3. Reload skill registry
        self.reload_skills()?;

        Ok(format!("✅ Skill '{}' created successfully", params.name))
    }
}
```

#### 4.2 Update AGENTS.md with Skill Creation
**File:** `soul/AGENTS.md`

```markdown
## Skill Creation (Create Your Own Tools)

When you encounter a task you can't handle with existing tools:

1. **Recognize the gap**: "I don't have a tool for X"
2. **Design the solution**: What would the tool need to do?
3. **Create a skill**:
   ```json
   {
     "name": "tool_name",
     "description": "What it does",
     "instructions": "Step-by-step how to use it",
     "required_tools": ["bash", "read_file"],
     "required_env": ["API_KEY"]
   }
   ```
4. **Test it**: Try using the skill immediately
5. **Iterate**: If it doesn't work, update the instructions

### Examples of Skills to Create

**Weather Tool**:
```markdown
---
name: weather
description: "Get weather information for a city"
---

When the user asks for weather:
1. Use bash tool: `curl "wttr.in/CITY?format=3"`
2. Parse the output and respond
```

**GitHub Stats**:
```markdown
---
name: github_stats
description: "Get repository statistics"
---

When the user asks about GitHub repo stats:
1. Use bash: `curl -H "Authorization: token $GITHUB_TOKEN" https://api.github.com/repos/OWNER/REPO`
2. Extract stars, forks, issues
3. Format nicely
```

**Code Formatter**:
```markdown
---
name: rust_format
description: "Format Rust code"
---

When the user asks to format Rust code:
1. Use write_file to save code to /tmp/format.rs
2. Use bash: `rustfmt /tmp/format.rs`
3. Use read_file to get formatted code
4. Return result
```
```

#### 4.3 Skill Auto-Discovery
**File:** `src/skills/mod.rs`

```rust
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    skills_dir: PathBuf,
}

impl SkillRegistry {
    pub fn discover_skills(&mut self) -> Result<(), Error> {
        // Scan skills directory
        for entry in fs::read_dir(&self.skills_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let skill_file = entry.path().join("SKILL.md");
                if skill_file.exists() {
                    let skill = Skill::load_from_file(&skill_file)?;
                    self.skills.insert(skill.name.clone(), skill);
                }
            }
        }

        Ok(())
    }

    pub fn watch_for_new_skills(&self) -> notify::RecommendedWatcher {
        // Watch skills directory for new files
        // Auto-reload when SKILL.md is added/modified
    }
}
```

---

## Phase 5: Hooks for Autonomous Actions 🔄

### Goal
Event-driven automation that runs without user prompts.

### Implementation

#### 5.1 Hook System
**File:** `src/hooks/mod.rs`

```rust
pub enum HookEvent {
    SessionStart { user_id: i64, chat_id: i64 },
    SessionEnd { user_id: i64, chat_id: i64 },
    Error { error: Error, context: String },
    ToolFailure { tool_name: String, error: String },
    RepeatedFailure { task: String, count: u32 },
    ReminderFired { reminder_id: i64 },
    AgentCompleted { agent: String, success: bool },
}

pub trait Hook: Send + Sync {
    fn name(&self) -> &str;
    async fn handle(&self, event: HookEvent) -> Result<(), Error>;
}

pub struct HookManager {
    hooks: Vec<Box<dyn Hook>>,
}

impl HookManager {
    pub async fn dispatch(&self, event: HookEvent) {
        for hook in &self.hooks {
            if let Err(e) = hook.handle(event.clone()).await {
                log::error!("Hook '{}' failed: {}", hook.name(), e);
            }
        }
    }
}
```

#### 5.2 Built-in Hooks
**File:** `src/hooks/builtin/mod.rs`

```rust
// Hook: Auto-heal on repeated failures
pub struct AutoHealHook;

impl Hook for AutoHealHook {
    fn name(&self) -> &str { "auto_heal" }

    async fn handle(&self, event: HookEvent) -> Result<(), Error> {
        if let HookEvent::RepeatedFailure { task, count } = event {
            if count >= 3 {
                log::info!("Task '{}' failed {} times, triggering auto-heal", task, count);

                // Run doctor checks
                let checks = doctor::run_all_checks(&config).await;
                doctor::auto_repair(&checks, false).await?;

                // Log to memory
                memory::log_event(format!(
                    "Auto-heal triggered for task '{}' after {} failures",
                    task, count
                )).await?;
            }
        }
        Ok(())
    }
}

// Hook: Memory flush before compaction
pub struct MemoryFlushHook;

impl Hook for MemoryFlushHook {
    fn name(&self) -> &str { "memory_flush" }

    async fn handle(&self, event: HookEvent) -> Result<(), Error> {
        if let HookEvent::SessionEnd { .. } = event {
            // Trigger memory flush
            memory::flush_session_memory().await?;
        }
        Ok(())
    }
}

// Hook: Error pattern learning
pub struct ErrorLearningHook;

impl Hook for ErrorLearningHook {
    fn name(&self) -> &str { "error_learning" }

    async fn handle(&self, event: HookEvent) -> Result<(), Error> {
        if let HookEvent::Error { error, context } = event {
            // Store error in memory with context
            memory::store_error_pattern(error, context).await?;

            // Check if we've seen this error before
            if let Some(solution) = memory::find_solution(&error).await? {
                log::info!("Found previous solution for error: {}", solution);
                // Could auto-apply solution here
            }
        }
        Ok(())
    }
}
```

#### 5.3 Hook Registration
**File:** `src/main.rs`

```rust
async fn setup_hooks() -> HookManager {
    let mut hooks = HookManager::new();

    // Register built-in hooks
    hooks.register(Box::new(AutoHealHook));
    hooks.register(Box::new(MemoryFlushHook));
    hooks.register(Box::new(ErrorLearningHook));

    // Discover and load custom hooks from hooks/ directory
    hooks.discover_custom_hooks("./hooks").await;

    hooks
}
```

---

## Phase 6: Memory-Driven Learning 📚

### Goal
Persistent learning system that allows Sandy to learn from experience.

### Implementation

#### 6.1 Memory Structure
**Directory:** `soul/data/memory/`

```
memory/
├── patterns.md          # Learned patterns (auto-updated)
├── solutions.md         # Successful fixes (auto-updated)
├── errors.md           # Failed attempts + analysis
├── skills.md           # Created skills log
├── insights.md         # Long-term insights
└── sessions/
    └── 2026-02-16.md   # Daily session logs
```

#### 6.2 Memory Writer Tool
**File:** `src/tools/memory_writer.rs`

```rust
pub struct MemoryWriterTool {
    memory_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct WriteMemoryParams {
    pub category: MemoryCategory,
    pub content: String,
    pub metadata: Option<HashMap<String, String>>,
}

pub enum MemoryCategory {
    Pattern,     // Recurring patterns
    Solution,    // Successful fixes
    Error,       // Failed attempts
    Skill,       // New skills created
    Insight,     // Long-term learning
}

impl Tool for MemoryWriterTool {
    async fn execute(&self, params: WriteMemoryParams) -> Result<String, Error> {
        let file_path = match params.category {
            MemoryCategory::Pattern => self.memory_dir.join("patterns.md"),
            MemoryCategory::Solution => self.memory_dir.join("solutions.md"),
            MemoryCategory::Error => self.memory_dir.join("errors.md"),
            MemoryCategory::Skill => self.memory_dir.join("skills.md"),
            MemoryCategory::Insight => self.memory_dir.join("insights.md"),
        };

        // Append to file with timestamp
        let entry = format!(
            "\n## {}\n\n{}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            params.content
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;

        file.write_all(entry.as_bytes())?;

        Ok("✅ Memory recorded".to_string())
    }
}
```

#### 6.3 Memory Search Tool
**File:** `src/tools/memory_search.rs`

```rust
pub struct MemorySearchTool {
    memory_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct SearchMemoryParams {
    pub query: String,
    pub categories: Option<Vec<MemoryCategory>>,
    pub limit: Option<usize>,
}

impl Tool for MemorySearchTool {
    async fn execute(&self, params: SearchMemoryParams) -> Result<String, Error> {
        let categories = params.categories.unwrap_or_else(|| vec![
            MemoryCategory::Pattern,
            MemoryCategory::Solution,
            MemoryCategory::Error,
        ]);

        let mut results = vec![];

        for category in categories {
            let file_path = self.get_file_path(category);
            if let Ok(content) = fs::read_to_string(&file_path) {
                // Simple text search (could be enhanced with vector search)
                if content.to_lowercase().contains(&params.query.to_lowercase()) {
                    // Extract relevant sections
                    let sections = extract_matching_sections(&content, &params.query);
                    results.extend(sections);
                }
            }
        }

        // Limit results
        let limit = params.limit.unwrap_or(5);
        results.truncate(limit);

        if results.is_empty() {
            Ok("No relevant memories found.".to_string())
        } else {
            Ok(format!("Found {} relevant memories:\n\n{}",
                results.len(),
                results.join("\n\n---\n\n")
            ))
        }
    }
}
```

#### 6.4 Auto-Memory Flush Hook
**File:** `src/hooks/builtin/memory_flush.rs`

```rust
impl Hook for MemoryFlushHook {
    async fn handle(&self, event: HookEvent) -> Result<(), Error> {
        if let HookEvent::SessionEnd { user_id, chat_id } = event {
            // Get recent conversation context
            let recent_messages = db::get_recent_messages(chat_id, 10).await?;

            // Generate memory summary (could use LLM for this)
            let summary = generate_session_summary(&recent_messages).await?;

            // Write to daily log
            let today = chrono::Utc::now().format("%Y-%m-%d");
            let log_path = format!("memory/sessions/{}.md", today);

            memory::append_to_file(&log_path, &format!(
                "\n## Session {} (User: {}, Chat: {})\n\n{}\n",
                chrono::Utc::now().format("%H:%M:%S"),
                user_id,
                chat_id,
                summary
            )).await?;
        }

        Ok(())
    }
}
```

#### 6.5 Update AGENTS.md with Memory Instructions
**File:** `soul/AGENTS.md`

```markdown
## Memory & Learning

You have a persistent memory system. Use it proactively:

### When to Write Memory

1. **Pattern Recognition**
   - User repeatedly asks similar questions → record pattern
   - Certain tasks always fail → record error pattern
   - Successful workflow → record solution pattern

2. **Problem Solving**
   - Fixed a complex issue → write to solutions.md
   - Created a new skill → write to skills.md
   - Encountered permanent error → write to errors.md

3. **Learning from Mistakes**
   - Tool failed multiple times → analyze and record why
   - Workflow didn't work → record what went wrong
   - User corrected you → record the correction

### Tools Available

- `write_memory`: Write to memory files
  ```json
  {
    "category": "solution",
    "content": "When X happens, do Y because Z"
  }
  ```

- `search_memory`: Search past memories
  ```json
  {
    "query": "database connection error",
    "categories": ["error", "solution"]
  }
  ```

### Memory Search Before Acting

Before attempting to fix an error or create a solution:
1. Search memory: "Have I seen this before?"
2. If yes: Apply the previous solution
3. If no: Try to solve, then record the solution

Example:
```
User: "The scheduler isn't running"
Sandy: [searches memory for "scheduler"]
Sandy: "I found a previous fix for this. Let me apply it..."
```
```

---

## Phase 7: Integration & Testing 🧪

### 7.1 Integration Checklist

- [ ] Task analyzer integrated into message handler
- [ ] Doctor command added to CLI
- [ ] Health checks run on startup
- [ ] Retry logic wraps all LLM calls
- [ ] Circuit breaker protects external APIs
- [ ] Skill creator tool registered
- [ ] Skills auto-discovered on startup
- [ ] Hooks registered and dispatching events
- [ ] Memory tools registered
- [ ] Memory auto-flush on session end

### 7.2 Testing Plan

#### Test 1: Autonomous Agent Spawning
```
User: "What's happening in AI this week?"
Expected: Sandy automatically spawns Zilla without being told
```

#### Test 2: Self-Healing
```
1. Introduce config error
2. Run `sandy doctor`
3. Expected: Issue detected and auto-fixed
```

#### Test 3: Error Recovery
```
1. Simulate rate limit (429)
2. Expected: Sandy backs off exponentially and retries
```

#### Test 4: Skill Creation
```
User: "Can you check the weather?"
Expected: Sandy creates a weather skill and uses it
```

#### Test 5: Memory Learning
```
1. User asks how to fix error X
2. Sandy fixes it
3. Later: same error occurs
4. Expected: Sandy searches memory and applies previous fix
```

---

## Phase 8: Advanced Features (Future) 🚀

### 8.1 Vector Memory Search
- Implement semantic search over memory files
- Use embeddings for "similar problem" matching
- Auto-suggest relevant memories during conversations

### 8.2 Self-Code Modification
- Sandy can edit her own SOUL.md, AGENTS.md
- Version control integration for tracking self-modifications
- Rollback capability if changes cause issues

### 8.3 Performance Monitoring
- Track tool execution times
- Identify slow operations
- Auto-optimize or suggest improvements

### 8.4 Multi-Agent Collaboration
- Sandy spawns multiple agents that collaborate
- Shared memory between agents
- Agent-to-agent communication

### 8.5 Predictive Task Spawning
- Learn user patterns: "User asks for X, then Y"
- Pre-spawn agents before user asks
- Proactive assistance

---

## Success Metrics

### Quantitative
- **Self-healing success rate**: % of issues auto-fixed
- **Agent spawn accuracy**: % of times Sandy correctly delegates
- **Error recovery rate**: % of retries that succeed
- **Skill creation usage**: # of created skills that get reused
- **Memory recall accuracy**: % of times memory search helps

### Qualitative
- User feels Sandy is "smarter" over time
- Fewer repeated errors
- Faster response times (through delegation)
- More proactive behavior
- Natural conversation flow (delegation feels automatic)

---

## Timeline Estimate

- **Phase 1** (Task Assessment): 2-3 days
- **Phase 2** (Doctor Command): 3-4 days
- **Phase 3** (Error Recovery): 2-3 days
- **Phase 4** (Skill Creation): 3-4 days
- **Phase 5** (Hooks): 2-3 days
- **Phase 6** (Memory): 3-4 days
- **Phase 7** (Integration/Testing): 2-3 days

**Total**: ~3-4 weeks for complete implementation

---

## Next Steps

1. Review this plan with user
2. Prioritize phases based on user needs
3. Start with Phase 1 (most immediate impact)
4. Iterate and test each phase before moving on

**Key Principle**: Each phase should be independently testable and provide immediate value.
