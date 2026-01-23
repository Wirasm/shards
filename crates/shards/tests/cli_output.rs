//! Integration tests for CLI output behavior

use std::process::Command;

/// Verify that stdout contains only user-facing output (no JSON logs)
#[test]
fn test_list_stdout_is_clean() {
    let output = Command::new(env!("CARGO_BIN_EXE_shards"))
        .arg("list")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // stdout should not contain JSON log lines
    assert!(
        !stdout.contains(r#""event":"#),
        "stdout should not contain JSON logs, got: {}",
        stdout
    );

    // stderr should contain JSON logs (if any logging occurred)
    // Note: logs go to stderr now
    if !stderr.is_empty() {
        // If there's output on stderr, it should be JSON logs
        assert!(
            stderr.contains(r#""timestamp""#) || stderr.contains(r#""level""#),
            "stderr should contain structured logs, got: {}",
            stderr
        );
    }
}

/// Verify piping works correctly
#[test]
fn test_output_is_pipeable() {
    let output = Command::new(env!("CARGO_BIN_EXE_shards"))
        .arg("list")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // stdout should be clean enough to pipe through grep
    // Every line should either be empty, the "No active shards" message,
    // "Active shards:" header, or table content (starts with special chars or |)
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Should not be JSON
        assert!(
            !trimmed.starts_with('{'),
            "stdout contains JSON line: {}",
            line
        );
    }
}
