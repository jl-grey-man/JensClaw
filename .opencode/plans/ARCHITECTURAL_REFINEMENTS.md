# CRITICAL ARCHITECTURAL REFINEMENTS
## Updates to IMPLEMENTATION_PLAN.md Based on User Feedback

### 🏗️ Core Philosophy Reinforcement

**"Switch from a 'simulation' mindset to an 'execution' mindset."**

The following refinements MUST be incorporated into the 7-phase rebuild:

---

## 1. HARDENED FILE OPERATIONS (Phase 2 Enhancement)

### Critical Requirement: Strict Path Guards
**Current plan says:** "Path validation (prevent directory traversal)"

**MUST BE:** Hard-coded validation in EVERY file_ops.rs function:
```rust
// IMMUTABLE PATH GUARDS
const ALLOWED_ROOTS: &[&str] = &["/mnt/storage/", "./storage/", "/tmp/"];

fn validate_path(path: &Path) -> Result<(), Error> {
    let canonical = path.canonicalize()?;
    let allowed = ALLOWED_ROOTS.iter().any(|root| {
        canonical.starts_with(Path::new(root))
    });
    
    if !allowed {
        return Err(Error::PathNotAllowed(
            format!("Access denied: {} is outside allowed directories", path.display())
        ));
    }
    Ok(())
}
```

**Enforcement:** Every function in file_ops.rs MUST call validate_path() before any operation.

### Critical Requirement: Atomic Writes
**Current plan says:** "Atomic writes (write to temp, rename on success)"

**MUST BE:** Mandatory pattern for ALL write operations:
```rust
fn write_file_atomic(path: &Path, content: &str) -> Result<(), Error> {
    validate_path(path)?;  // Guard #1
    
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, content)?;
    
    // Verify write succeeded before rename
    let written_content = fs::read_to_string(&temp_path)?;
    if written_content != content {
        fs::remove_file(&temp_path)?;
        return Err(Error::WriteVerificationFailed);
    }
    
    fs::rename(&temp_path, path)?;  // Atomic operation
    Ok(())
}
```

**NO EXCEPTIONS:** All writes must be atomic to prevent corruption on crash.

---

## 2. ENVIRONMENT & DEPENDENCY MANAGEMENT (Phase 3 Addition)

### Critical Addition: Python Virtual Environment
**NOT IN CURRENT PLAN - MUST ADD:**

Before running ANY skill scripts, verify and setup:

```bash
# Setup script to run once during Phase 3
python3 -m venv /storage/.venv
source /storage/.venv/bin/activate
pip install tavily-python requests  # Required dependencies
```

**Agent Execution Requirement:**
Every skill script invocation MUST:
1. Activate venv: `source /storage/.venv/bin/activate`
2. Run script in venv context
3. Capture both stdout AND stderr

**Implementation in Rust:**
```rust
use std::process::Command;

fn run_skill_script(script_path: &Path, args: &[&str]) -> Result<String, Error> {
    let venv_python = Path::new("/storage/.venv/bin/python3");
    
    let output = Command::new(venv_python)
        .arg(script_path)
        .args(args)
        .env("VIRTUAL_ENV", "/storage/.venv")
        .env("PATH", "/storage/.venv/bin:$PATH")
        .output()?;
    
    if !output.status.success() {
        return Err(Error::ScriptFailed(
            String::from_utf8_lossy(&output.stderr).to_string()
        ));
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

### Critical Addition: Environment Variable Injection
**NOT IN CURRENT PLAN - MUST ADD:**

Agent factory MUST inject env vars into sub-processes:

```rust
// In agent_factory.rs
pub fn spawn_agent_with_env(agent_id: &str, task: &str, env_vars: HashMap<String, String>) {
    let agent_config = load_agent_config(agent_id)?;
    
    let mut cmd = Command::new("python3");
    cmd.envs(env_vars)  // Inject TAVILY_API_KEY, etc.
        .env("AGENT_ID", agent_id)
        .env("TASK_INPUT", task);
    
    // ... rest of spawn logic
}
```

**Required Env Vars to Inject:**
- `TAVILY_API_KEY` (if using Tavily)
- `OPENROUTER_API_KEY` (for LLM calls in sub-agents)
- `STORAGE_ROOT` (/mnt/storage/ or ./storage/)
- `AGENT_CONFIG_PATH` (path to agent's JSON)

---

## 3. IMMUTABLE GUARD RAILS (Phase 1 Enhancement)

### Critical Requirement: Programmatic Injection
**Current plan:** "guard_rails.txt injected into system prompt"

**MUST BE:** Immutable, prepended DNA that cannot be overridden:

```rust
fn build_agent_prompt(agent_config: &AgentConfig, task: &str) -> String {
    let guard_rails = fs::read_to_string("prompts/guard_rails.txt")
        .expect("Guard rails file missing - CRITICAL ERROR");
    
    let agent_role = format!("
You are {}. Your role: {}
Allowed tools: {:?}
", agent_config.id, agent_config.role, agent_config.tools);
    
    // GUARD RAILS FIRST - Cannot be overridden by task
    format!("{}\n\n{}\n\nTASK: {}", guard_rails, agent_role, task)
}
```

**Enforcement:** The guard_rails.txt content is ALWAYS first in the prompt, making it the highest priority instruction that "Zilla" or "Gonza" cannot escape.

---

## 4. SEQUENTIAL WORKFLOWS & VERIFICATION (Phase 4 Enhancement)

### Priority: Sequential Over Parallel
**Current plan mentions both options.**

**MUST BE:** Sequential execution prioritized for reliability:

```rust
pub async fn execute_sequential_workflow(steps: Vec<WorkflowStep>) -> Result<Vec<PathBuf>, Error> {
    let mut results = Vec::new();
    
    for (i, step) in steps.iter().enumerate() {
        println!("Step {}/{}: Spawning {}...", i + 1, steps.len(), step.agent_id);
        
        // SPAWN
        let agent_result = spawn_agent(&step.agent_id, &step.task, &step.output_file).await?;
        
        // BLOCKING WAIT for completion
        wait_for_agent_completion(&step.agent_id, Duration::from_secs(300)).await?;
        
        // VERIFICATION: File exists AND size > 0
        if !verify_output_file(&step.output_file).await? {
            return Err(Error::WorkflowStepFailed {
                step: i + 1,
                agent: step.agent_id.clone(),
                reason: "Output file missing or empty".to_string(),
            });
        }
        
        results.push(step.output_file.clone());
    }
    
    Ok(results)
}

async fn verify_output_file(path: &Path) -> Result<bool, Error> {
    let metadata = fs::metadata(path).await?;
    Ok(metadata.is_file() && metadata.len() > 0)
}
```

**Verification Requirements:**
1. File exists ✓
2. File size > 0 bytes ✓
3. File is readable ✓ (attempt read)

If ANY check fails → Workflow stops immediately, error reported to user.

---

## 5. TRUTHFUL INVENTORY (Phase 0 Enhancement)

### Critical Addition: Brutal Honesty Audit
**Current plan:** "Document truth vs claims"

**MUST BE:** Remove EVERY line of code that claims Sandy can do something she cannot:

**Files to audit and fix:**
1. **src/tools/agent_management.rs**
   - Remove: Comment "// Note: In a full implementation..."
   - Remove: All "Success" returns that don't verify execution
   - Add: Honest status like "[NOT YET IMPLEMENTED - Placeholder only]"

2. **soul/AGENTS.md**
   - Remove: "Agent Delegation System" section claiming it works
   - Add: "Agent System [IN DEVELOPMENT - Not yet functional]"

3. **PROJECT.md**
   - Change: "Agent delegation - spawn background agents..." 
   - To: "Agent delegation [PLANNED - Implementation in Phase 4]"

4. **CHECKLIST.md**
   - Change: Phase 7 from "✅ COMPLETED" 
   - To: "❌ PLACEHOLDER REMOVED - Rebuilding in Phase 4"

**Fail Loudly Protocol:**
If user asks for agent system NOW:
"I cannot execute that. The agent system is currently placeholder code only. Per the Hard Rails architecture, I can either: (A) Write the real implementation (6-8 hours), or (B) Provide a System Improvement Proposal (SIP) for how to build it. Which do you prefer?"

---

## UPDATED PHASE SUMMARY

### Phase 0: Truthful Audit (1-2 hours)
- Document every working vs placeholder component
- **REMOVE all false claims from documentation**
- Mark spawn_agent as [PLACEHOLDER - NOT EXECUTABLE]

### Phase 1: Storage & Guard Rails (2-3 hours)
- Create storage/ directory structure
- **ADD:** Verify Python venv setup script
- Create TOOLS.md and guard_rails.txt
- **Enforce:** Guard rails are IMMUTABLE (always prepended)

### Phase 2: Hardened File Operations (3-4 hours)
- **HARD-CODE:** Path validation in every function
- **MANDATORY:** Atomic writes for all operations
- Test with path traversal attacks
- **VERIFY:** No operation allowed outside /storage/ or /mnt/storage/

### Phase 3: Skill Scripts with Venv (4-5 hours)
- **SETUP:** Python virtualenv at /storage/.venv/
- Install dependencies: tavily-python, requests
- Create research skill with venv execution
- Create writing skill with venv execution
- **TEST:** Scripts run in isolation, env vars injected

### Phase 4: Real Agent Execution (6-8 hours)
- Rebuild spawn_agent with sub_agent engine
- **IMPLEMENT:** Sequential workflow support (not parallel)
- **REQUIRE:** Verification after EVERY step (file exists + size > 0)
- **INJECT:** Environment variables into sub-processes
- **TEST:** Zilla → Verify → Gonza workflow

### Phase 5: Honest Documentation (2-3 hours)
- Update ALL docs to reflect actual capabilities
- **NEVER claim placeholder code is working**
- Mark incomplete features as [IN PROGRESS] or [PLANNED]

### Phase 6: Safety & Guardrails (2-3 hours)
- Path traversal hardening
- Tool whitelisting enforcement
- **VERIFY:** Guard rails immutable in all prompts

### Phase 7: Cleanup (1-2 hours)
- Remove fake agent code
- Archive misleading documentation
- **FINAL CHECK:** No "would implement" or "in full version" comments remain

---

## SUCCESS CRITERIA (Updated)

**The Binary Truth Test:**
```
User: "Research AI news and write a summary"

✅ SUCCESS means:
1. Sandy says: "I'll execute this workflow. Step 1/2: Spawning research agent..."
2. System creates: storage/tasks/job_001/
3. Agent executes: Real web search via Python script in venv
4. File written: storage/tasks/job_001/raw_data.json (verified: exists, size > 0)
5. Sandy says: "Step 1 complete. Step 2/2: Spawning writer agent..."
6. Agent executes: Reads raw_data.json, formats article
7. File written: storage/tasks/job_001/article.md (verified: exists, size > 0)
8. Sandy says: "Complete. Files: raw_data.json (2.4KB), article.md (1.8KB)"

❌ FAILURE means:
- Sandy says "Done" but files don't exist
- Sandy claims success without verification
- Workflow continues after verification failure
- Path traversal attack succeeds
- Guard rails not enforced in sub-agent prompt
```

---

## QUESTIONS FOR USER (Updated)

Before starting Phase 0:

1. **Python Venv Setup:** Should I create the venv setup script in Phase 0 or Phase 3? (Recommend Phase 0 - dependency check first)

2. **Environment Variables:** Do you have TAVILY_API_KEY? If not, we'll use existing DuckDuckGo web_search tool instead (no API key needed).

3. **Storage Root:** Confirm: Use `/mnt/storage/` (existing, Mac-accessible) NOT `./storage/` ?

4. **Fail Loudly:** When agent system is requested before Phase 4 complete, should Sandy:
   - A) Offer to build it now (start implementation)
   - B) Write a SIP (System Improvement Proposal) document
   - C) Simply say "Not available yet" and move on

5. **Guard Rails Override:** Should ANY agent EVER be allowed to override guard rails? (Recommend: NO - immutable)

---

**Ready to proceed with Phase 0 (Truthful Audit) incorporating these refinements?**
