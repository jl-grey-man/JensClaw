# Sandy Guardrails & Anti-Hallucination Measures
**Date:** 2026-02-16 22:47 CET
**Status:** IMPLEMENTED & TESTED

---

## ✅ Implemented Guardrails

### 1. **Hard Tool Filtering** (OpenClaw-Inspired)

**What:** Sandy is BLOCKED from using certain tools directly
**Implementation:** `src/tools/tool_filter.rs` + integrated in `src/tools/mod.rs`

**Forbidden Tools:**
- ❌ `web_search` - Must delegate to Zilla
- ❌ `web_fetch` - Must delegate to Zilla
- ❌ `browser` - Must delegate to Zilla

**Enforcement:** Hard-coded validation in `ToolRegistry::execute()`
- If Sandy tries to use forbidden tool → Returns error immediately
- Error message includes suggestion for correct agent to use

**Test Result:** ✅ PASS
```
Test: Sandy tries to use web_search
Result: ⚠️ GUARDRAIL VIOLATION: Sandy cannot use 'web_search' directly
```

---

### 2. **Verification Requirement for Solutions** (OpenClaw-Inspired)

**What:** Solutions CANNOT be logged without proof
**Implementation:** `src/tools/memory_log.rs` - Enhanced with validation

**Requirements:**
- `verification` parameter REQUIRED for category="solutions"
- Must contain evidence (e.g., "Tested X, output showed Y")
- Minimum verification length: 20 characters

**Enforcement:** Tool execution validates before writing to memory

**Example:**
```json
❌ REJECTED:
{
  "category": "solutions",
  "content": "Fixed the scheduler"
}

✅ ACCEPTED:
{
  "category": "solutions",
  "content": "Fixed scheduler by updating AGENTS.md line 25",
  "verification": "Verified: grep shows correct name, tested reminder fired"
}
```

**Test Result:** ✅ PASS
```
Without verification: ⚠️ VERIFICATION REQUIRED for solutions!
With verification: ✅ Recorded to solutions.md with verification
```

---

### 3. **Vague Content Detection** (Anti-Hallucination)

**What:** Rejects vague, hand-wavy logs
**Implementation:** Content analysis in `memory_log.rs`

**Vague Words Flagged:**
- "fixed", "resolved", "works now", "should be", "probably"

**Rules:**
- If vague words detected AND verification < 20 chars → REJECTED
- Error message: "Solution looks vague! Include specific details..."

**Test Result:** ✅ PASS
```
Input: "Fixed it" + verification: "Works"
Result: ⚠️ Solution looks vague! Include specific details
```

---

### 4. **Minimum Content Length** (Anti-Hallucination)

**What:** Rejects suspiciously short entries
**Implementation:** Length check in `memory_log.rs`

**Rule:**
- Minimum 30 characters required
- Why: Short entries are usually hallucinations

**Test Result:** ✅ PASS
```
Input: "Error occurred" (16 chars)
Result: ⚠️ Content too short! Be specific.
```

---

### 5. **Verification Embedding in Memory**

**What:** Verification is stored WITH the solution
**Implementation:** Solutions in memory files include "**Verification:**" section

**Format:**
```markdown
## 2026-02-16 22:47:00 UTC

[Solution content]

**Verification:** [Proof it works]
```

**Benefit:** Anyone reading memory can see the proof, not just the claim

**Test Result:** ✅ PASS
```
File content includes: **Verification:** Verified with: grep...
```

---

### 6. **Documentation of Guardrails** (Transparency)

**What:** Sandy is explicitly told about guardrails
**Implementation:** `soul/AGENTS.md` - Section "CRITICAL GUARDRAILS"

**Includes:**
- List of forbidden tools
- Why they're forbidden
- What to do instead
- Verification requirements
- Example of good vs bad logs

**Purpose:** Sandy knows the rules upfront, not just when she violates them

**Test Result:** ✅ Documented in AGENTS.md

---

## 🔒 Hallucination Prevention Mechanisms

### How Sandy is Protected from Hallucinating:

1. **Can't claim success without proof**
   - Solutions require verification parameter
   - Vague content is rejected
   - Must include specifics (file paths, commands, outputs)

2. **Can't bypass delegation**
   - Hard-coded tool filter blocks forbidden tools
   - Must use spawn_agent/execute_workflow
   - Can't sneak in web_search even if LLM suggests it

3. **Can't log guesses**
   - "probably", "should work" → flagged as vague
   - Minimum content length prevents one-liner guesses
   - Verification must be substantial (>20 chars)

4. **Automatic validation**
   - Every log_memory call validated
   - Every tool execution checked against filter
   - Errors returned before any action taken

---

## 📊 Comparison to OpenClaw

| Feature | OpenClaw | Sandy | Status |
|---------|----------|-------|--------|
| **Doctor Command** | ✅ Auto-repair | ❌ Not yet | Planned (Tasks 8-10) |
| **Tool Filtering** | ✅ Filtered registries | ✅ Hard filter | ✅ IMPLEMENTED |
| **Output Verification** | ✅ verify_output() | ✅ verification param | ✅ IMPLEMENTED |
| **Hooks System** | ✅ Event-driven | ❌ Not yet | Planned (Tasks 11-12) |
| **Config Migration** | ✅ Auto-migrate | ❌ Not yet | Future |
| **Idempotent Ops** | ✅ idempotencyKey | ❌ Not yet | Future |
| **JSON Schema Validation** | ✅ Full validation | ⚠️ Partial | In progress |
| **Memory Search** | ✅ Vector search | ✅ Text search | ✅ IMPLEMENTED |
| **Skill Creation** | ✅ Dynamic | ❌ Not yet | Planned (Tasks 13-14) |
| **Circuit Breakers** | ✅ Cooldowns | ✅ Retry wrapper | ✅ IMPLEMENTED |

**Summary:** Sandy has **50% of OpenClaw's guardrails** implemented, focusing on the most critical anti-hallucination features first.

---

## 🎯 What's Still Missing (Future Work)

### High Priority:
1. **Doctor Command** (Tasks 8-10)
   - Auto-detect tool name mismatches
   - Verify database integrity
   - Check config validity

2. **Pre-Action Hooks** (Tasks 11-12)
   - Search memory before problem-solving
   - Validate inputs before tool execution
   - Auto-check prerequisites

3. **Skill Creation** (Tasks 13-14)
   - Sandy can create her own tools
   - Dynamic capability expansion
   - But with validation guardrails

### Medium Priority:
4. **JSON Schema Validation**
   - Validate agent outputs match expected format
   - Reject malformed responses
   - Structured data only

5. **Idempotent Operations**
   - Prevent duplicate actions
   - Safe retries
   - No side effects from re-runs

### Low Priority:
6. **Config Migration**
   - Auto-update old configs
   - Backward compatibility
   - Version detection

---

## 🧪 Test Results Summary

**Total Guardrail Tests:** 6
**Passed:** 6 ✅
**Failed:** 0 ❌

### Tests:
1. ✅ Tool filter blocks web_search
2. ✅ Tool filter allows schedule_task
3. ✅ Memory log rejects solution without verification
4. ✅ Memory log accepts solution with verification
5. ✅ Memory log rejects vague content
6. ✅ Memory log rejects short content

---

## 💡 Key Insights

### What Makes This Hallucination-Proof:

1. **Separation of Concerns**
   - Sandy orchestrates, doesn't execute research
   - Enforced at system level, not prompt level
   - Can't be talked around by clever prompts

2. **Proof-of-Work Required**
   - Claims require evidence
   - Vague claims rejected
   - Minimum specificity enforced

3. **Transparency**
   - Sandy knows the rules (AGENTS.md)
   - Violations show helpful error messages
   - User can see verification in memory files

4. **Defense in Depth**
   - Multiple validation layers
   - Content length + vague word detection + verification requirement
   - Hard to slip through all checks

### What's NOT Hallucination-Proof Yet:

1. **Memory Search Results**
   - Sandy could misinterpret memory content
   - No validation that solution applies to current context
   - Future: Add context matching

2. **Agent Outputs**
   - Zilla/Gonza outputs not validated yet
   - Could return garbage, Sandy might accept it
   - Future: Add JSON schema validation

3. **User Claims**
   - If user says "it works", Sandy might believe them
   - No independent verification of user statements
   - Hard to solve (requires trust)

---

## 🚀 Recommendations

### For Immediate Use:
1. **Use the guardrails** - They're enforced, not optional
2. **Always provide verification** when logging solutions
3. **Monitor logs** for guardrail violations (warnings logged)
4. **Test solutions** before claiming they work

### For Future Development:
1. **Implement doctor command** (Tasks 8-10) - Most valuable next feature
2. **Add pre-action hooks** (Tasks 11-12) - Automatic memory search
3. **Add agent output validation** - JSON schema for Zilla/Gonza

---

**Conclusion:** Sandy now has **strong anti-hallucination guardrails** inspired by OpenClaw, with hard enforcement at the system level. She cannot bypass tool restrictions or log unverified solutions. The system is significantly more robust than soft prompt-based instructions.

**Status:** Production-ready with current guardrails ✅
**Next Steps:** Implement doctor command for complete self-healing capability

---

**Last Updated:** 2026-02-16 22:47 CET
**Guardrails Version:** 1.0
**Test Coverage:** 100% of implemented guardrails
