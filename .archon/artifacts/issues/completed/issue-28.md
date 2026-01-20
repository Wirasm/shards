# Investigation: Improve agent process detection reliability

**Issue**: #28 (https://github.com/Wirasm/shards/issues/28)
**Type**: BUG
**Investigated**: 2026-01-20T15:13:35.679+02:00

### Assessment

| Metric | Value | Reasoning |
|--------|-------|-----------|
| Severity | MEDIUM | Feature partially broken with moderate impact - health monitoring shows incorrect status but core functionality (session creation) still works |
| Complexity | MEDIUM | 2-3 files affected with some integration points, moderate risk due to cross-platform process detection |
| Confidence | HIGH | Clear root cause identified in find_process_by_name() function with strong evidence from code analysis and issue description |

---

## Problem Statement

Agent process detection frequently fails, causing all sessions to show as "Crashed" in health monitoring even when agents are running successfully. The current implementation has timing issues, process name mismatches, and insufficient search patterns.

---

## Analysis

### Root Cause / Change Rationale

The process detection system has multiple failure points that compound to create unreliable agent process tracking:

### Evidence Chain

WHY: Health dashboard shows all sessions as ❌ Crashed
↓ BECAUSE: `enrich_session_with_metrics()` determines `process_running = false`
  Evidence: `src/health/handler.rs:65` - `match process::is_process_running(pid)`

↓ BECAUSE: Sessions have `process_id = None` due to failed process detection during spawn
  Evidence: `src/terminal/handler.rs:68-76` - Process search returns `None`, logs "Agent process not found"

↓ BECAUSE: `find_process_by_name()` uses exact string matching and insufficient timing
  Evidence: `src/process/operations.rs:194-220` - `process_name.contains(name_pattern)` with 500ms delay

↓ ROOT CAUSE: Multiple detection weaknesses compound to create failures
  Evidence: 
  - `src/terminal/operations.rs:105` - `extract_command_name("kiro-cli chat")` returns "kiro-cli" but process might be named differently
  - `src/core/config.rs:88` - Fixed 500ms delay insufficient for agent startup
  - `src/process/operations.rs:207` - Single-pass search with no retry logic

### Affected Files

| File | Lines | Action | Description |
|------|-------|--------|-------------|
| `src/process/operations.rs` | 194-220 | UPDATE | Improve find_process_by_name with better patterns and retry logic |
| `src/terminal/operations.rs` | 105-107 | UPDATE | Enhance extract_command_name to handle more agent patterns |
| `src/core/config.rs` | 88 | UPDATE | Increase default spawn delay and make it configurable per agent |
| `src/terminal/handler.rs` | 61-76 | UPDATE | Add retry logic with exponential backoff for process detection |

### Integration Points

- `src/health/handler.rs:65` depends on process_id being set correctly
- `src/sessions/handler.rs` stores the detected process_id in session metadata
- `src/cli/commands.rs:362` health command displays the results to users

### Git History

- **Introduced**: 8c3ec36 - 2026-01-19 - "Fix: Track agent process instead of osascript launcher (#13)"
- **Last modified**: 568ed29 - 2026-01-20 - "feat: Add shards health command with process monitoring (#18)"
- **Implication**: Recent feature that fixed one issue but introduced timing/pattern matching problems

---

## Implementation Plan

### Step 1: Improve process search patterns

**File**: `src/process/operations.rs`
**Lines**: 194-220
**Action**: UPDATE

**Current code:**
```rust
// Line 194-220
pub fn find_process_by_name(
    name_pattern: &str,
    command_pattern: Option<&str>,
) -> Result<Option<ProcessInfo>, ProcessError> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    for (pid, process) in system.processes() {
        let process_name = process.name().to_string_lossy();
        
        if !process_name.contains(name_pattern) {
            continue;
        }

        if let Some(cmd_pattern) = command_pattern {
            let cmd_line = process.cmd().iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");
            if !cmd_line.contains(cmd_pattern) {
                continue;
            }
        }

        return Ok(Some(ProcessInfo {
            pid: Pid::from_raw(pid.as_u32()),
            name: process_name.to_string(),
            status: ProcessStatus::from(process.status()),
            start_time: process.start_time(),
        }));
    }

    Ok(None)
}
```

**Required change:**
```rust
pub fn find_process_by_name(
    name_pattern: &str,
    command_pattern: Option<&str>,
) -> Result<Option<ProcessInfo>, ProcessError> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    // Try multiple search strategies
    let search_patterns = generate_search_patterns(name_pattern);
    
    for (pid, process) in system.processes() {
        let process_name = process.name().to_string_lossy();
        let cmd_line = process.cmd().iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        
        // Try each search pattern
        let name_matches = search_patterns.iter().any(|pattern| {
            process_name.contains(pattern) || cmd_line.contains(pattern)
        });
        
        if !name_matches {
            continue;
        }

        // If command pattern specified, it must match
        if let Some(cmd_pattern) = command_pattern {
            if !cmd_line.contains(cmd_pattern) {
                continue;
            }
        }

        return Ok(Some(ProcessInfo {
            pid: Pid::from_raw(pid.as_u32()),
            name: process_name.to_string(),
            status: ProcessStatus::from(process.status()),
            start_time: process.start_time(),
        }));
    }

    Ok(None)
}

/// Generate multiple search patterns for better process matching
fn generate_search_patterns(name_pattern: &str) -> Vec<String> {
    let mut patterns = vec![name_pattern.to_string()];
    
    // Add partial matches
    if name_pattern.contains('-') {
        patterns.push(name_pattern.split('-').next().unwrap_or(name_pattern).to_string());
    }
    
    // Add common variations
    match name_pattern {
        "kiro-cli" => patterns.extend(vec!["kiro".to_string(), "kiro-cli".to_string()]),
        "claude-code" => patterns.extend(vec!["claude".to_string(), "claude-code".to_string()]),
        "gemini-cli" => patterns.extend(vec!["gemini".to_string(), "gemini-cli".to_string()]),
        _ => {}
    }
    
    patterns
}
```

**Why**: Current exact matching fails when process names don't match command names exactly

---

### Step 2: Add retry logic with exponential backoff

**File**: `src/terminal/handler.rs`
**Lines**: 61-76
**Action**: UPDATE

**Current code:**
```rust
// Line 61-76
    // Wait for terminal to spawn the agent process
    let delay_ms = config.terminal.spawn_delay_ms;
    info!(event = "terminal.waiting_for_agent_spawn", delay_ms, command);
    std::thread::sleep(std::time::Duration::from_millis(delay_ms));

    // Try to find the actual agent process
    let agent_name = operations::extract_command_name(command);
    let (process_id, process_name, process_start_time) = 
        match crate::process::find_process_by_name(&agent_name, Some(command)) {
            Ok(Some(info)) => (Some(info.pid.as_u32()), Some(info.name), Some(info.start_time)),
            _ => {
                warn!(
                    event = "terminal.agent_process_not_found",
                    agent_name, command,
                    message = "Agent process not found - session created but process tracking unavailable"
                );
                (None, None, None)
            }
        };
```

**Required change:**
```rust
    // Try to find the actual agent process with retry logic
    let agent_name = operations::extract_command_name(command);
    let (process_id, process_name, process_start_time) = 
        find_agent_process_with_retry(&agent_name, command, &config)?;
```

**Why**: Single attempt with fixed delay is insufficient for reliable process detection

---

### Step 3: Add retry helper function

**File**: `src/terminal/handler.rs`
**Lines**: NEW
**Action**: CREATE

**Required change:**
```rust
/// Find agent process with retry logic and exponential backoff
fn find_agent_process_with_retry(
    agent_name: &str,
    command: &str,
    config: &ShardsConfig,
) -> Result<(Option<u32>, Option<String>, Option<u64>), TerminalError> {
    let max_attempts = 5;
    let mut delay_ms = config.terminal.spawn_delay_ms;
    
    for attempt in 1..=max_attempts {
        info!(
            event = "terminal.searching_for_agent_process",
            attempt,
            max_attempts,
            delay_ms,
            agent_name,
            command
        );
        
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        
        match crate::process::find_process_by_name(agent_name, Some(command)) {
            Ok(Some(info)) => {
                info!(
                    event = "terminal.agent_process_found",
                    attempt,
                    pid = info.pid.as_u32(),
                    process_name = info.name,
                    agent_name
                );
                return Ok((Some(info.pid.as_u32()), Some(info.name), Some(info.start_time)));
            }
            Ok(None) => {
                if attempt == max_attempts {
                    warn!(
                        event = "terminal.agent_process_not_found_final",
                        agent_name,
                        command,
                        attempts = max_attempts,
                        message = "Agent process not found after all retry attempts - session created but process tracking unavailable"
                    );
                } else {
                    info!(
                        event = "terminal.agent_process_not_found_retry",
                        attempt,
                        max_attempts,
                        agent_name,
                        next_delay_ms = delay_ms * 2
                    );
                }
            }
            Err(e) => {
                warn!(
                    event = "terminal.agent_process_search_error",
                    attempt,
                    agent_name,
                    error = %e
                );
            }
        }
        
        // Exponential backoff: 500ms, 1s, 2s, 4s, 8s
        delay_ms *= 2;
    }
    
    Ok((None, None, None))
}
```

**Why**: Provides multiple attempts with increasing delays to handle variable agent startup times

---

### Step 4: Increase default spawn delay

**File**: `src/core/config.rs`
**Lines**: 88
**Action**: UPDATE

**Current code:**
```rust
// Line 88
            spawn_delay_ms: 500,
```

**Required change:**
```rust
            spawn_delay_ms: 1000,
```

**Why**: 500ms is often insufficient for agent processes to fully initialize

---

### Step 5: Add tests for improved process detection

**File**: `src/process/operations.rs`
**Lines**: NEW
**Action**: CREATE

**Test cases to add:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_search_patterns() {
        let patterns = generate_search_patterns("kiro-cli");
        assert!(patterns.contains(&"kiro-cli".to_string()));
        assert!(patterns.contains(&"kiro".to_string()));
        
        let patterns = generate_search_patterns("claude-code");
        assert!(patterns.contains(&"claude-code".to_string()));
        assert!(patterns.contains(&"claude".to_string()));
    }

    #[test]
    fn test_find_process_by_name_with_partial_match() {
        // This would need a running process to test properly
        // For now, just ensure the function doesn't panic
        let result = find_process_by_name("nonexistent", None);
        assert!(result.is_ok());
    }
}
```

---

## Patterns to Follow

**From codebase - mirror these exactly:**

```rust
// SOURCE: src/process/operations.rs:160-190
// Pattern for comprehensive testing with process lifecycle
#[test]
fn test_process_lifecycle() {
    let mut child = Command::new("sleep")
        .arg("10")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn test process");

    let pid = child.id();

    let is_running = is_process_running(pid).expect("Failed to check process");
    assert!(is_running);

    let info = get_process_info(pid).expect("Failed to get process info");
    assert_eq!(info.pid.as_u32(), pid);
    assert!(info.name.contains("sleep"));

    let kill_result = kill_process(pid, Some(&info.name), Some(info.start_time));
    assert!(kill_result.is_ok());

    let _ = child.kill();
    let _ = child.wait();
}
```

```rust
// SOURCE: src/terminal/handler.rs:61-63
// Pattern for structured logging with event names
info!(event = "terminal.waiting_for_agent_spawn", delay_ms, command);
std::thread::sleep(std::time::Duration::from_millis(delay_ms));
```

---

## Edge Cases & Risks

| Risk/Edge Case | Mitigation |
|----------------|------------|
| Process starts after max retry attempts | Log clear warning, session still created but without PID tracking |
| Multiple processes match search pattern | Return first match (existing behavior), add logging to show which was selected |
| Agent process exits immediately after start | Health monitoring will detect this in next check cycle |
| Platform-specific process naming differences | Search patterns include common variations for known agents |
| High CPU usage from frequent process scanning | Use shared system instance and limit refresh frequency |

---

## Validation

### Automated Checks

```bash
cargo test process::operations::test_find_process_by_name
cargo test process::operations::test_generate_search_patterns
cargo test terminal::handler
cargo clippy
```

### Manual Verification

1. Create a shard with `shards create test-branch --agent kiro` and verify process is detected
2. Run `shards health` and confirm session shows as ✅ Working instead of ❌ Crashed
3. Test with different agents (claude, gemini) to ensure patterns work
4. Verify retry logic by monitoring logs during agent startup

---

## Scope Boundaries

**IN SCOPE:**
- Improving process detection reliability during session creation
- Adding retry logic with exponential backoff
- Better search patterns for known agents
- Enhanced logging for debugging process detection issues

**OUT OF SCOPE (do not touch):**
- Changing the overall health monitoring architecture
- Modifying session persistence format
- Adding new agent types beyond pattern improvements
- Real-time process monitoring (separate from startup detection)

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-01-20T15:13:35.679+02:00
- **Artifact**: `.archon/artifacts/issues/issue-28.md`
