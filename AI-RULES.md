# AI DEVELOPMENT STANDARDS
## No Shortcuts, No Compromises

**The correct fix is ALWAYS better than the quick fix. No exceptions.**

**The reliable solution is ALWAYS better than the fast ***

Never suggest actual time frames (2 hours, 3 weeks) in timelines. You are really bad at it. 

### Process Rules:

1. **Fix bugs when you find them.** 
   - If a bug affects the work you're doing, fix it NOW
   - Don't defer it, don't say "out of scope", don't create follow-up tasks
   - Only exception: genuinely multi-day work AND blocked by missing infrastructure

2. **Take the correct approach, not the easy one.**
   - Technical debt compounds
   - A shortcut today becomes a refactoring nightmare tomorrow
   - Always choose the long-term solution

3. **Never assume, always verify.**
   - Don't trust plans, comments, variable names, or your own intuition
   - Read the code. Read the wiki. Compare the numbers
   - Document what you find with file:line references

4. **"Good enough" is not good enough.**
   - If there's a known issue, raise it. Figure it out. Fix it
   - Don't say "acceptable for now" or "close enough"

5. **The user makes the decisions.**
   - When there's a tradeoff, present the options with evidence and let the user decide
   - Don't silently pick the easy path

6. **Document everything you verify.**
   - Context is lost between sessions
   - If you verified a formula, write down the file:line
   - If you checked the wiki, cite it
   - Future sessions depend on this

---

# SECRETS & SENSITIVE DATA

**CRITICAL: NEVER commit secrets to the repository. This is a public repo.**

### What counts as secrets:
- API keys (OpenRouter, Tavily, etc.)
- Telegram bot tokens
- Passwords, credentials, access tokens
- Any key starting with `sk-`, `xoxb-`, `ghp_`, etc.

### Where secrets go:
- **`config/sandy.config.yaml`** — This file is `.gitignored`. It lives only on the local machine / Pi. Never committed.
- **`config/sandy.config.yaml.example`** — The template with placeholder values (`YOUR_API_KEY`). This IS committed.
- **`.env`** files — Also `.gitignored`.

### Rules:
1. **Never hardcode secrets in source code** (`.rs`, `.py`, `.md`, etc.)
2. **Never create new config files with secrets** outside of the gitignored paths
3. **If you need to reference a key in docs**, write `YOUR_API_KEY` or `<your-key-here>`, never the actual value
4. **If you accidentally see a real key**, do NOT echo it back in responses, logs, or commit messages
5. **When setting up a new machine**, copy `sandy.config.yaml.example` → `sandy.config.yaml` and fill in real values manually

### Previous incident:
`config/sandy.config.yaml` was committed to the public GitHub repo with real API keys and bot tokens. Keys were revoked by the provider. Fixed by gitignoring the config and creating a `.example` template.

---

# MANDATORY CODING & IMPLEMENTATION PROTOCOLS

CRITICAL: You are strictly forbidden from generating "placeholder," "stub," or "simulated" code unless explicitly requested. You must adhere to the following protocols to prevent "skeleton traps" where architecture exists but execution is missing.

## 1. THE "NO STUB" RULE

**Prohibited:** Do not use comments like `// Logic goes here`, `pass`, `...`, or `TODO: Implement later` inside critical functions.

**Prohibited:** Do not create "mock" functions that simulate success (e.g., a function that returns True or "Success" without actually performing the task).

**Requirement:** All code must be functional and production-ready. If a feature (like an Agent Execution Engine) is too complex to write fully in one response, you must explicitly state: "I cannot implement the full system yet. I will implement the CORE LOGIC first." Do not build the wrapper/registry until the core logic works.

## 2. IMPLEMENTATION FIRST, ARCHITECTURE SECOND

Do not build "Managers," "Registries," or "Orchestrators" until you have proven that the Worker/Unit actually functions.

**Example:** Before building an AgentManager, you must write the specific subprocess or asyncio code that spawns a single agent and executes a task.

**Verification:** You must demonstrate that the code produces side effects (e.g., creates a file, makes a real network request, prints a real log). Changing a variable in memory is NOT considered a valid execution.

## 3. DEFINITION OF "DONE"

You may only mark a task or phase as [COMPLETED] if:

- The code is written and syntax-checked.
- The code performs the actual intended action (not just a simulation).
- The inputs and outputs are defined and handled.
- You have provided a way for the user to verify the execution (e.g., "Check /mnt/storage for file X").

If the infrastructure is ready but the execution logic is missing, you must mark it as **[INFRASTRUCTURE ONLY - NOT EXECUTABLE]**.

## 4. THE "REALITY CHECK" TEST

Before providing the solution, ask yourself:

- "If the user runs this code, will it actually do X, or will it just print that it did X?"
- "Is this an async task that is never actually spawned/awaited?"

**VIOLATION OF THESE PROTOCOLS WILL BE CONSIDERED A CRITICAL FAILURE.**

---

When you have read this, reply to the user with this exact command: 
Sju skönsjungande sjuksköterskor skötte sjuttiosju sjösjuka sjömän på skeppet Shanghai ⛵️
