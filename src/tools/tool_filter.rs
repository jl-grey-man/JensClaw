/// Tool filtering and validation to enforce guardrails
use std::collections::HashSet;
use std::sync::LazyLock;

/// Sandy's forbidden tools (static, allocated once)
static FORBIDDEN_FOR_SANDY: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("web_search");
    s.insert("web_fetch");
    s.insert("browser");
    s
});

/// Tools that require verification before execution
static REQUIRES_VERIFICATION: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("log_memory");
    s.insert("bash");
    s.insert("write_file");
    s.insert("edit_file");
    s
});

/// Check if Sandy (main bot) is allowed to use this tool directly
pub fn can_sandy_use(tool_name: &str) -> Result<(), String> {
    if FORBIDDEN_FOR_SANDY.contains(tool_name) {
        return Err(format!(
            "⚠️ GUARDRAIL VIOLATION: Sandy cannot use '{}' directly. \
             You must delegate this to a specialized agent. \
             \nUse spawn_agent or execute_workflow instead. \
             \n- For research/web: spawn Zilla \
             \n- For writing: spawn Gonza",
            tool_name
        ));
    }
    Ok(())
}

/// Check if this tool requires verification
pub fn requires_verification(tool_name: &str) -> bool {
    REQUIRES_VERIFICATION.contains(tool_name)
}

/// Get a helpful error message for forbidden tool usage
pub fn get_alternative(tool_name: &str) -> String {
    match tool_name {
        "web_search" | "web_fetch" | "browser" => {
            "Instead of using this tool directly, spawn Zilla: \
            spawn_agent(agent_name='Zilla', task='your research task', \
            output_file='/mnt/storage/tasks/research.json')".to_string()
        }
        _ => format!("Tool '{}' should be delegated to an appropriate agent", tool_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandy_forbidden_tools() {
        assert!(can_sandy_use("web_search").is_err());
        assert!(can_sandy_use("web_fetch").is_err());
        assert!(can_sandy_use("browser").is_err());
    }

    #[test]
    fn test_sandy_allowed_tools() {
        assert!(can_sandy_use("schedule_task").is_ok());
        assert!(can_sandy_use("spawn_agent").is_ok());
        assert!(can_sandy_use("read_file").is_ok());
    }

    #[test]
    fn test_verification_required() {
        assert!(requires_verification("log_memory"));
        assert!(requires_verification("bash"));
        assert!(!requires_verification("read_file"));
    }
}
