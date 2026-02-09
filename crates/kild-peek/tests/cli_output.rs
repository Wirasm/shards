//! Integration tests for kild-peek CLI output behavior
//!
//! The default behavior is quiet (no logs). Use -v/--verbose to enable logs.

use std::process::Command;

/// Execute 'kild-peek list windows' and verify it succeeds
fn run_peek_list_windows() -> std::process::Output {
    let output = Command::new(env!("CARGO_BIN_EXE_kild-peek"))
        .args(["list", "windows"])
        .output()
        .expect("Failed to execute 'kild-peek list windows'");

    assert!(
        output.status.success(),
        "kild-peek list windows failed with exit code {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    output
}

/// Execute 'kild-peek -v list windows' (verbose mode) and return the output
fn run_peek_verbose_list_windows() -> std::process::Output {
    let output = Command::new(env!("CARGO_BIN_EXE_kild-peek"))
        .args(["-v", "list", "windows"])
        .output()
        .expect("Failed to execute 'kild-peek -v list windows'");

    assert!(
        output.status.success(),
        "kild-peek -v list windows failed with exit code {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    output
}

// =============================================================================
// Default Mode (Quiet) Behavioral Tests
// =============================================================================

/// Verify that default mode (no flags) suppresses all log levels
#[test]
fn test_default_mode_suppresses_all_logs() {
    let output = run_peek_list_windows();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should NOT contain any log level — quiet mode is OFF, not ERROR
    for level in &["INFO", "DEBUG", "WARN", "ERROR", "TRACE"] {
        let pattern = format!(r#""level":"{}""#, level);
        assert!(
            !stderr.contains(&pattern),
            "Default mode should suppress {} logs, but stderr contains: {}",
            level,
            stderr
        );
    }
}

/// Verify that stdout contains only user-facing output (no JSON logs)
#[test]
fn test_stdout_is_clean() {
    let output = run_peek_list_windows();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // stdout should not contain JSON log lines
    assert!(
        !stdout.contains(r#""event":"#),
        "stdout should not contain JSON logs, got: {}",
        stdout
    );
}

// =============================================================================
// Verbose Mode Behavioral Tests
// =============================================================================

/// Verify verbose mode (-v) emits INFO logs
#[test]
fn test_verbose_flag_emits_info_logs() {
    let output = run_peek_verbose_list_windows();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verbose mode should contain INFO-level log events
    assert!(
        stderr.contains(r#""level":"INFO""#),
        "Verbose mode should emit INFO logs, but stderr is: {}",
        stderr
    );
}

/// Verify verbose mode works with --verbose long form
#[test]
fn test_verbose_flag_long_form_emits_logs() {
    let output = Command::new(env!("CARGO_BIN_EXE_kild-peek"))
        .args(["--verbose", "list", "windows"])
        .output()
        .expect("Failed to execute 'kild-peek --verbose list windows'");

    assert!(
        output.status.success(),
        "kild-peek --verbose list windows failed with exit code {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains(r#""level":"INFO""#),
        "--verbose long form should emit INFO logs, but stderr is: {}",
        stderr
    );
}

// =============================================================================
// Clean Error Output Tests
// =============================================================================

/// Verify that error output in default mode contains ONLY the user-facing message
#[test]
fn test_error_output_is_clean_in_default_mode() {
    let output = Command::new(env!("CARGO_BIN_EXE_kild-peek"))
        .args(["diff", "/nonexistent/img1.png", "/nonexistent/img2.png"])
        .output()
        .expect("Failed to execute 'kild-peek diff'");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain the user-facing error message
    assert!(
        stderr.contains("Failed"),
        "Should contain user-facing error message, got: {}",
        stderr
    );

    // Should NOT contain JSON log lines
    assert!(
        !stderr.contains(r#""level":"ERROR""#),
        "Default mode should suppress ERROR JSON logs, got: {}",
        stderr
    );

    // Should NOT contain raw Rust Debug representation
    assert!(
        !stderr.contains("Error: "),
        "Should not show Rust Debug error representation, got: {}",
        stderr
    );
}

/// Verify that verbose mode shows JSON logs on error
#[test]
fn test_error_output_verbose_shows_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_kild-peek"))
        .args([
            "-v",
            "diff",
            "/nonexistent/img1.png",
            "/nonexistent/img2.png",
        ])
        .output()
        .expect("Failed to execute 'kild-peek -v diff'");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain user-facing error
    assert!(
        stderr.contains("Failed"),
        "Should contain user-facing error message, got: {}",
        stderr
    );

    // Should contain JSON error logs in verbose mode
    assert!(
        stderr.contains(r#""level":"ERROR""#),
        "Verbose mode should show ERROR JSON logs, got: {}",
        stderr
    );
}
