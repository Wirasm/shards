//! Integration tests for CLI output behavior

use std::process::Command;

/// Verify that stdout contains only user-facing output (no JSON logs)
/// and that any stderr output is structured JSON logs.
#[test]
fn test_list_stdout_is_clean() {
    let output = Command::new(env!("CARGO_BIN_EXE_shards"))
        .arg("list")
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "test_list_stdout_is_clean: Failed to execute 'shards list': {}",
                e
            )
        });

    // Verify command succeeded before examining output
    assert!(
        output.status.success(),
        "shards list failed with exit code {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // stdout should not contain JSON log lines
    assert!(
        !stdout.contains(r#""event":"#),
        "stdout should not contain JSON logs, got: {}",
        stdout
    );

    // stderr should contain JSON logs (if any logging occurred)
    if !stderr.is_empty() {
        // If there's output on stderr, it should be JSON logs
        assert!(
            stderr.contains(r#""timestamp""#) || stderr.contains(r#""level""#),
            "stderr should contain structured logs, got: {}",
            stderr
        );
    }
}

/// Verify stdout has no JSON lines and is suitable for piping
#[test]
fn test_output_is_pipeable() {
    let output = Command::new(env!("CARGO_BIN_EXE_shards"))
        .arg("list")
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "test_output_is_pipeable: Failed to execute 'shards list': {}",
                e
            )
        });

    // Verify command succeeded before examining output
    assert!(
        output.status.success(),
        "shards list failed with exit code {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // stdout should be clean enough to pipe through grep
    // No line should be JSON (starting with '{')
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
