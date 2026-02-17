# Sandy Agent System Analysis Report
**Date:** 2026-02-16
**Status:** Phase 4 Complete (Infrastructure) - Execution Issues Identified
**Severity:** HIGH - System functional but not operating as designed

---

## Executive Summary

**Problem:** Sandy is an orchestrator agent designed to delegate work to specialized sub-agents (Zilla for research, Gonza for writing), but she's doing all the work herself instead of delegating.

**Root Cause:** Sandy doesn't know the agent delegation tools exist. They're registered in the code but **not documented in her system prompt** (AGENTS.md). She can't use tools she doesn't know about.

**Impact:**
- Multi-agent workflows don't happen naturally
- Sandy does research and writing directly instead of delegating
- The expensive agent infrastructure is effectively unused
- Cost optimization through specialized agents not realized

**Fix Complexity:** MEDIUM - Requires documentation updates and some code fixes, but no major architectural changes needed.

---

## Critical Findings

### 🔴 CRITICAL: Missing Tool Documentation

**Issue:** The agent delegation tools are not documented in AGENTS.md (Sandy's capability reference).

**What's Missing:**
```markdown
| spawn_agent       | Delegate task to specialized agent |
| execute_workflow  | Run multi-step agent workflows     |
| list_agents       | View active agent jobs             |
| agent_status      | Check agent job status             |
| create_agent_config | Define new agent types           |
```

**What's In AGENTS.md:**
- Only 11 basic tools (bash, files, web, memory, skills)
- No mention of agent delegation capabilities
- No orchestration protocols

**Evidence:**
- `/home/jens/sandy/soul/AGENTS.md:11-23` - Complete tool list, agents missing
- `/home/jens/sandy/src/tools/mod.rs:150-156` - Tools ARE registered in code

**Why This Breaks Delegation:**
Sandy's behavior is guided by her system prompt (SOUL.md + AGENTS.md). If tools aren't documented, she won't think to use them. The LLM needs explicit guidance on available capabilities.

---

### 🔴 CRITICAL: No Orchestration Instructions

**Issue:** Sandy's personality files don't teach her to delegate or orchestrate work.

**Current Identity (SOUL.md):**
- "You are Sandy, a personal assistant and accountability partner"
- Focus on ADHD coaching, pattern learning, task management
- No mention of agent delegation or workflow orchestration

**Missing Instructions:**
- When to spawn agents vs. doing work directly
- How to use execute_workflow for multi-step tasks
- Best practices for agent delegation
- Verification protocols after agent execution

**Example of What's Needed:**
```markdown
## Work Delegation

When the user asks you to research something:
1. Use spawn_agent with agent_id="zilla" to delegate research
2. Wait for completion and verify output file exists
3. Read the results and summarize for the user

For multi-step tasks (research → write):
1. Use execute_workflow with steps for Zilla → Gonza
2. Workflow handles verification between steps
3. Report final results to user
```

**Evidence:**
- `/home/jens/sandy/soul/SOUL.md` - No orchestration guidance
- `/home/jens/sandy/architecture.md:51-70` - Architecture defines orchestration model but Sandy never reads this file

---

### 🟡 HIGH: Tool Filtering Not Implemented

**Issue:** Agent configs specify `allowed_tools` but they're not enforced.

**The Problem:**
```rust
// agent_management.rs:175-198
fn create_filtered_registry(&self, allowed_tools: &[String]) -> ToolRegistry {
    // This function exists but is NEVER CALLED
    // Dead code warning: "method `create_filtered_registry` is never used"
}
```

**Current Behavior:**
- spawn_agent loads agent config (with tools: ["web_search", "write_file", ...])
- Builds task prompt with guard rails
- Calls sub_agent with **FULL** new_sub_agent registry (11 tools)
- Agent config's `allowed_tools` is ignored

**What Should Happen:**
- Load agent config
- Create filtered registry with ONLY allowed_tools
- Pass filtered registry to sub_agent
- Agent cannot access tools not in their config

**Evidence:**
- `/home/jens/sandy/src/tools/agent_management.rs:175` - Dead code warning
- `/home/jens/sandy/src/tools/agent_management.rs:321-328` - Uses full sub_agent, no filtering
- `/home/jens/sandy/storage/agents/zilla.json:6-12` - Specifies tools but they're not enforced

**Security Impact:** MEDIUM
- Agents can access more tools than intended
- Gonza (writer) could do web searches (shouldn't have access)
- Violates principle of least privilege

---

### 🟡 MEDIUM: No JSON Output Validation

**Issue:** Agents specify output_format but it's not validated.

**Current Verification:**
```rust
// agent_management.rs:201-215
async fn verify_output(&self, output_path: &str) -> Result<(bool, u64), String> {
    // Checks:
    // 1. File exists
    // 2. Size > 0
    // 3. Content doesn't start with "ERROR:"

    // MISSING: Validate it's actually valid JSON when format is "structured_json"
}
```

**The Risk:**
- Zilla config: `"output_format": "structured_json"`
- Agent could write plain text, markdown, or malformed JSON
- Sandy would accept it as valid output
- Downstream consumers (Gonza) would fail when trying to parse

**Example Failure:**
```
Zilla saves: "Here's what I found: ..." (plain text)
Verification: ✅ PASS (file exists, size > 0, no ERROR prefix)
Sandy: "Research complete!"
User: "Now write an article"
Gonza: Tries to parse JSON → FAILS
```

**Evidence:**
- `/home/jens/sandy/storage/agents/zilla.json:21` - Specifies JSON output
- `/home/jens/sandy/src/tools/agent_management.rs:201-215` - No format validation
- `/home/jens/sandy/src/tools/execute_workflow.rs:80-91` - Checks for ERROR but not format

---

### 🟢 LOW: Weak Guard Rails

**Issue:** Guard rails are generic and don't prevent hallucination effectively.

**Current Guard Rails:**
```
*** CRITICAL SAFETY PROTOCOLS ***
1. YOU ARE RESTRICTED: You are a specialized sub-process.
2. FILE SYSTEM ONLY: Perform your task and save results to file.
3. NO CHIT-CHAT: Do not converse. Output only work product.
4. SOURCE OF TRUTH: If you cannot complete, write ERROR to file.
5. TOOL RESTRICTIONS: You can only use allowed tools.
6. OUTPUT REQUIRED: You MUST save work to specified file.
```

**Problems:**
- Point 5 mentions "allowed tools" but no enforcement mechanism
- No JSON validation instruction
- No examples of correct output format
- No explicit anti-hallucination protocol

**Recommended Additions:**
```
7. NO HALLUCINATION: Use tools to get real data. Never make up facts.
8. OUTPUT FORMAT: Save results as valid JSON with this structure: {...}
9. VERIFICATION: After writing file, read it back to verify format.
10. ERROR PROTOCOL: If task fails, write JSON: {"error": "message"}
```

**Evidence:**
- `/home/jens/sandy/src/tools/agent_management.rs:138-144` - Current guard rails
- `/home/jens/sandy/architecture.md:95-102` - More detailed example in docs but not used

---

## Architecture Review

### What Works ✅

1. **Core Infrastructure:**
   - spawn_agent tool loads configs and creates job folders correctly
   - sub_agent execution engine works (spawns LLM subprocess)
   - Agent registry tracks job status
   - execute_workflow handles sequential steps with verification
   - File operations are atomic and path-validated
   - Code compiles with only minor warnings

2. **Agent Configs:**
   - Well-structured JSON with roles, tools, constraints
   - Zilla (research) and Gonza (writer) are properly defined
   - Output formats specified
   - Constraints documented

3. **Workflow System:**
   - execute_workflow parses steps correctly
   - Stops on verification failure (fail-fast)
   - Passes output files between steps
   - Good error reporting

### What's Broken ❌

1. **Discovery:** Sandy doesn't know agent tools exist
2. **Orchestration:** No guidance on when/how to delegate
3. **Tool Filtering:** Agent-specific tool restrictions not enforced
4. **Format Validation:** Output format not verified
5. **Guard Rails:** Generic, not hallucination-proof

---

## Efficiency Analysis

### Current Tool Registry

**Main Sandy Registry (16 tools):**
```
- Agent Factory, Spawn Agent, List Agents, Agent Status, Execute Workflow
- Bash, Browser, Read/Write/Edit File, Glob, Grep
- Memory, Web Fetch, Web Search, Activate Skill
```

**Sub-Agent Registry (11 tools):**
```
- Bash, Browser, Read/Write/Edit File, Glob, Grep
- Memory, Web Fetch, Web Search, Activate Skill
(Missing: Agent tools - correct, prevents recursion)
```

**Zilla Should Have (5 tools):**
```
- web_search, web_fetch, write_file, read_file, bash
```

**Gonza Should Have (2 tools):**
```
- read_file, write_file
```

**Current Problem:**
Both Zilla and Gonza get all 11 sub-agent tools, not their restricted set.

---

## JSON Hallucination Prevention

### Current Risk: HIGH

**Why JSON Matters:**
- Zilla outputs structured research data for Gonza to parse
- Invalid JSON breaks the workflow
- LLMs can hallucinate JSON structure
- Need validation at write time, not parse time

**Current Process:**
```
1. Zilla does research
2. Writes output (no format validation)
3. System checks file exists + size > 0
4. ✅ Marked as complete
5. Gonza tries to parse → may fail
```

**Recommended Process:**
```
1. Zilla does research
2. Writes output to temp file
3. System validates JSON structure
4. If valid → rename to final path
5. If invalid → ERROR state, retry required
6. ✅ Only mark complete if JSON validates
```

### Implementation Strategy

**Option 1: Post-Write Validation (Recommended)**
```rust
async fn verify_output(&self, output_path: &str, expected_format: &str) -> Result<(bool, u64), String> {
    // Existing checks: file exists, size > 0, no ERROR prefix

    // NEW: Format-specific validation
    if expected_format == "structured_json" || expected_format == "json" {
        let content = tokio::fs::read_to_string(output_path).await?;
        serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| format!("Invalid JSON output: {}", e))?;
    }

    Ok((true, size))
}
```

**Option 2: Schema Validation (Advanced)**
```rust
// Define expected schema in agent config
{
  "output_format": "structured_json",
  "output_schema": {
    "type": "object",
    "required": ["query", "results", "sources"]
  }
}

// Validate against schema
fn validate_json_schema(content: &str, schema: &Value) -> Result<(), String> {
    // Use jsonschema crate
}
```

---

## Recommended Fixes (Priority Order)

### 🔴 PHASE 1: Make Sandy Aware (1-2 hours)

**Goal:** Document agent tools so Sandy knows she can delegate.

**Tasks:**
1. Update `soul/AGENTS.md` with agent tools:
   ```markdown
   | spawn_agent | Spawn specialized agent to complete task |
   | execute_workflow | Run sequential multi-agent workflow |
   | list_agents | View active and completed agent jobs |
   | agent_status | Check status of specific agent job |
   | create_agent_config | Define new agent type with tool restrictions |
   ```

2. Add orchestration section to `soul/AGENTS.md`:
   ```markdown
   ## Work Delegation Protocols

   ### When to Delegate
   - Research tasks → spawn_agent(agent_id="zilla", ...)
   - Writing from research → spawn_agent(agent_id="gonza", input_file=...)
   - Multi-step workflows → execute_workflow([zilla_step, gonza_step])

   ### Verification Protocol
   1. Spawn agent and wait for completion
   2. Check agent_status for success/failure
   3. Read output file to verify content
   4. Report results to user
   ```

3. Update `soul/SOUL.md` to include orchestrator role:
   ```markdown
   ## Work Management

   You are an orchestrator. For complex tasks:
   - Delegate to specialized agents (Zilla for research, Gonza for writing)
   - Verify agent outputs before reporting success
   - Use workflows for multi-step processes
   ```

**Validation:**
- Ask Sandy: "Research recent AI news and write a summary article"
- Expected: She uses execute_workflow with Zilla → Gonza
- Current: She does research and writing herself

---

### 🟡 PHASE 2: Implement Tool Filtering (2-3 hours)

**Goal:** Enforce agent-specific tool restrictions.

**Tasks:**
1. Fix `create_filtered_registry` to actually filter:
   ```rust
   fn create_filtered_registry(&self, allowed_tools: &[String]) -> ToolRegistry {
       let mut registry = ToolRegistry::empty(); // NEW: Start with empty
       let base_registry = ToolRegistry::new_sub_agent(&self.config);

       for tool in base_registry.tools() {
           if allowed_tools.contains(&tool.name().to_string()) {
               registry.add_tool(tool);
           }
       }

       registry
   }
   ```

2. Use filtered registry in spawn_agent:
   ```rust
   // Line 318-328 in agent_management.rs
   let filtered_registry = self.create_filtered_registry(&agent_config.tools);

   // Create sub_agent with filtered registry (need to modify SubAgentTool to accept registry)
   let sub_agent_result = sub_agent_tool.execute_with_registry(
       sub_agent_input,
       filtered_registry
   ).await;
   ```

3. Add test to verify tool filtering:
   ```rust
   #[tokio::test]
   async fn test_agent_tool_filtering() {
       // Create config with only web_search
       let config = AgentConfig {
           tools: vec!["web_search".to_string()],
           ...
       };

       let registry = create_filtered_registry(&config.tools);
       assert_eq!(registry.tool_count(), 1);
       assert!(registry.has_tool("web_search"));
       assert!(!registry.has_tool("bash")); // Should not have bash
   }
   ```

**Validation:**
- Spawn Gonza (writer) with tools: [read_file, write_file]
- Attempt to use web_search → Should FAIL
- Expected error: "Tool 'web_search' not available to this agent"

---

### 🟡 PHASE 3: Add JSON Validation (1-2 hours)

**Goal:** Prevent invalid JSON output from passing verification.

**Tasks:**
1. Add format validation to `verify_output`:
   ```rust
   async fn verify_output(
       &self,
       output_path: &str,
       expected_format: &str
   ) -> Result<(bool, u64, Option<String>), String> {
       // Existing checks...

       // Format-specific validation
       match expected_format {
           "structured_json" | "json" => {
               let content = tokio::fs::read_to_string(output_path).await?;
               if let Err(e) = serde_json::from_str::<serde_json::Value>(&content) {
                   return Ok((false, size, Some(format!("Invalid JSON: {}", e))));
               }
           }
           "markdown" | "markdown_article" => {
               // Could add markdown validation if needed
           }
           _ => {} // No validation for unknown formats
       }

       Ok((true, size, None))
   }
   ```

2. Pass output_format to verify_output:
   ```rust
   // Line 331 in agent_management.rs
   let (output_exists, output_size) = match self.verify_output(
       output_path,
       &agent_config.output_format // NEW: Pass format
   ).await {
       ...
   }
   ```

3. Update guard rails to emphasize JSON:
   ```rust
   let guard_rails = format!(r#"*** CRITICAL SAFETY PROTOCOLS ***
   ...
   6. OUTPUT FORMAT: Your output MUST be valid {}. Verify before completing.
   7. VALIDATION: After writing, read back and parse to ensure format is correct.
   8. NO HALLUCINATION: All data must come from tool execution, not memory.
   "#, agent_config.output_format);
   ```

**Validation:**
- Spawn Zilla to research something
- Manually corrupt output file: `echo "invalid json" > output.json`
- Re-verify → Should detect invalid JSON
- Expected: Status changes to Failed("Invalid JSON: expected value at line 1...")

---

### 🟢 PHASE 4: Enhanced Guard Rails (1 hour)

**Goal:** Stronger hallucination prevention and error handling.

**Tasks:**
1. Create format-specific guard rail templates:
   ```rust
   fn build_guard_rails(agent_config: &AgentConfig) -> String {
       let base_rails = r#"*** CRITICAL SAFETY PROTOCOLS ***
       1. RESTRICTED AGENT: You are a specialized subprocess with limited tools.
       2. TOOL-ONLY DATA: All information must come from tool execution.
       3. NO HALLUCINATION: Never make up facts, URLs, or data.
       4. FILE OUTPUT REQUIRED: Save results to specified file path.
       5. ERROR PROTOCOL: If task fails, write {"error": "reason"} to output.
       "#;

       let format_specific = match agent_config.output_format.as_str() {
           "structured_json" | "json" => {
               r#"
       6. JSON OUTPUT: Save results as valid JSON. Structure: {"query": "...", "results": [...]}
       7. VALIDATION: Read file back and parse JSON before completing.
       8. SOURCE URLS: Include complete URLs for all sources."#
           }
           "markdown_article" | "markdown" => {
               r#"
       6. MARKDOWN OUTPUT: Use proper headings (##), lists, and formatting.
       7. CITE SOURCES: Include [Title](URL) links for all facts.
       8. STRUCTURE: Use logical sections with clear headings."#
           }
           _ => ""
       };

       format!("{}\n{}", base_rails, format_specific)
   }
   ```

2. Add constraint enforcement:
   ```rust
   let constraints_text = if !agent_config.constraints.is_empty() {
       format!("\n\n*** YOUR CONSTRAINTS (MUST FOLLOW) ***\n{}",
           agent_config.constraints.iter()
               .map(|c| format!("- {}", c))
               .collect::<Vec<_>>()
               .join("\n"))
   } else {
       String::new()
   };
   ```

3. Include examples in prompt:
   ```rust
   let example_output = match agent_config.output_format.as_str() {
       "structured_json" => {
           r#"

   Example output format:
   {
     "query": "AI news",
     "results": [
       {
         "title": "...",
         "url": "https://...",
         "summary": "...",
         "date": "2026-02-16"
       }
     ],
     "sources_count": 5
   }"#
       }
       _ => ""
   };
   ```

**Validation:**
- Spawn Zilla without examples → Check if JSON structure varies
- Spawn Zilla with examples → Check if structure is consistent
- Expected: More consistent, predictable output

---

## Testing Strategy

### Integration Tests Needed

**Test 1: Basic Delegation**
```rust
#[tokio::test]
async fn test_sandy_delegates_research() {
    // Setup: Sandy receives "Research AI news"
    // Expected: Uses spawn_agent(agent_id="zilla", ...)
    // Validation: Agent registry shows Zilla job created
}
```

**Test 2: Workflow Execution**
```rust
#[tokio::test]
async fn test_research_to_article_workflow() {
    // Setup: execute_workflow with Zilla → Gonza
    // Expected:
    //   - Zilla creates research.json (valid JSON)
    //   - Verification passes
    //   - Gonza reads research.json
    //   - Gonza creates article.md
    //   - Workflow reports success
}
```

**Test 3: Tool Filtering**
```rust
#[tokio::test]
async fn test_gonza_cannot_access_web() {
    // Setup: Spawn Gonza, have it try web_search
    // Expected: Tool execution fails with "Tool not available"
}
```

**Test 4: JSON Validation**
```rust
#[tokio::test]
async fn test_invalid_json_rejected() {
    // Setup: Mock agent writes invalid JSON
    // Expected: verify_output returns (false, size, Some("Invalid JSON..."))
}
```

**Test 5: Error Propagation**
```rust
#[tokio::test]
async fn test_workflow_stops_on_failure() {
    // Setup: Zilla fails to produce output
    // Expected: Workflow stops, Gonza never runs
}
```

---

## Performance & Cost Optimization

### Current Costs (Estimated)

**Scenario: "Research AI news and write article"**

**Current (Sandy does everything):**
- Input: System prompt (5KB SOUL + AGENTS) + User message + Tools (16 defs)
- Output: Research results + Article text
- Token estimate: ~3K input, ~1.5K output = **4.5K tokens**
- Cost: ~$0.015 per request (Sonnet 4.5 pricing)

**With Agent Delegation (Broken - Not Happening):**
- Sandy input: System prompt + Tools + User message = ~3K
- Sandy output: Delegation plan = ~0.2K
- Zilla input: Guard rails + Task = ~0.5K
- Zilla output: JSON results = ~1K
- Gonza input: Guard rails + Task + Research file = ~1.2K
- Gonza output: Article = ~0.8K
- **Total: ~6.7K tokens** (MORE than current!)

**Why This Is Wrong:**
Agent delegation INCREASES cost because:
1. Each agent spawn = separate LLM call = separate system prompt
2. Sandy still needs full context
3. Overhead from guard rails and task context

**When Agents Actually Help:**
- **Long-running tasks** - Agent works while Sandy handles other users
- **Specialized models** - Zilla uses cheaper model for research
- **Parallel execution** - Multiple agents work simultaneously
- **Caching** - Guard rails cached across multiple agent spawns

**Optimization Strategy:**
1. Use Haiku for research agents (10x cheaper)
2. Cache guard rails (90% reduction on repeated spawns)
3. Run agents in parallel when possible
4. Only delegate truly independent tasks

---

## Recommended Next Steps

### Immediate (Today)

1. **Update AGENTS.md** with agent tools and orchestration protocols
2. **Update SOUL.md** to include orchestrator role
3. **Test basic delegation:**
   ```
   User: "Research quantum computing news"
   Expected: Sandy uses spawn_agent(agent_id="zilla", ...)
   ```

### Short-term (This Week)

1. **Implement tool filtering** (Phase 2)
2. **Add JSON validation** (Phase 3)
3. **Write integration tests** for workflows
4. **Monitor agent usage** to verify delegation is happening

### Medium-term (Next Week)

1. **Enhanced guard rails** (Phase 4)
2. **Cost optimization** - Use Haiku for agents
3. **Agent prompt caching** to reduce costs
4. **Add more agent types** (code-writer, file-organizer, etc.)

### Long-term (This Month)

1. **Schema validation** for JSON outputs
2. **Agent templates** for common workflows
3. **Performance monitoring** dashboard
4. **User-facing agent status** in Telegram

---

## Success Metrics

### How to Know It's Working

**Metric 1: Delegation Rate**
- Track: spawn_agent calls / total user requests
- Target: >50% for tasks involving "research", "write", "analyze"
- Current: ~0% (Sandy does everything herself)

**Metric 2: Workflow Success Rate**
- Track: execute_workflow successes / total attempts
- Target: >90% success rate
- Current: Unknown (workflows not being used)

**Metric 3: Output Validation**
- Track: JSON validation failures / total agent outputs
- Target: <5% failure rate
- Current: No validation (100% pass regardless of format)

**Metric 4: Tool Restriction Violations**
- Track: Attempted tool uses outside allowed_tools
- Target: 0 (all attempts blocked)
- Current: No enforcement (agents have access to all tools)

---

## Risk Assessment

### High Risk ✅ (Addressed by Fixes)

1. **Sandy doesn't delegate** → Update AGENTS.md (Phase 1)
2. **Tool access not restricted** → Implement filtering (Phase 2)
3. **Invalid JSON accepted** → Add validation (Phase 3)

### Medium Risk ⚠️ (Monitoring Needed)

1. **Cost increase from delegation** → Optimize with Haiku + caching
2. **Workflow failures** → Add comprehensive error handling
3. **Agent prompt drift** → Periodic guard rail review

### Low Risk 🟢 (Acceptable)

1. **Minor compilation warnings** → Clean up later
2. **Dead code** → Remove after filtering implemented
3. **Documentation out of sync** → Update as part of Phase 1

---

## Conclusion

The agent system **infrastructure is complete and functional**, but Sandy isn't using it because:

1. **She doesn't know it exists** (not in AGENTS.md)
2. **She isn't taught to orchestrate** (no delegation protocols)
3. **Tool restrictions aren't enforced** (security issue)
4. **Output validation is weak** (hallucination risk)

**Good News:**
- Code compiles and runs
- Agent execution engine works
- Workflow system is solid
- Fixes are straightforward (mostly documentation + validation)

**Priority:**
- Phase 1 (documentation) is CRITICAL and fast (1-2 hours)
- Phase 2-3 (filtering + validation) are important for robustness
- Phase 4 (guard rails) is polish

**Recommendation:** Start with Phase 1 immediately. Test delegation behavior. Then proceed with Phase 2-3 for security and reliability.

---

**Report End**
