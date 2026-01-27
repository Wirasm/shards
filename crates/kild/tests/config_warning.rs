//! Integration tests for config warning behavior.
//!
//! These tests verify that the CLI properly warns users when config files have errors.

use std::fs;
use std::process::Command;

/// Test that an invalid config file produces a warning in stderr.
///
/// Note: We use the `create` command because it's one of the commands that loads config.
/// The `list` command doesn't load config, so it won't trigger warnings.
#[test]
fn test_config_warning_on_invalid_toml() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_dir = temp_dir.path().join(".kild");
    fs::create_dir_all(&config_dir).expect("Failed to create .kild dir");

    // Create an invalid TOML config file
    fs::write(config_dir.join("config.toml"), "invalid toml [[[")
        .expect("Failed to write invalid config");

    // Run kild create command (loads config via load_config_with_warning)
    // The command will fail because we're not in a git repo, but we should still see the warning
    let output = Command::new(env!("CARGO_BIN_EXE_kild"))
        .current_dir(temp_dir.path())
        .args(["create", "test-branch"])
        .output()
        .expect("Failed to execute kild");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify warning is shown (before the command fails for other reasons)
    assert!(
        stderr.contains("Warning: Could not load config"),
        "Expected warning in stderr, got: {}",
        stderr
    );

    // Verify the tip is shown
    assert!(
        stderr.contains("Tip: Check"),
        "Expected tip about config files in stderr, got: {}",
        stderr
    );
}

/// Test that a valid config file does not produce warnings.
#[test]
fn test_no_warning_on_valid_config() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_dir = temp_dir.path().join(".kild");
    fs::create_dir_all(&config_dir).expect("Failed to create .kild dir");

    // Create a valid TOML config file
    fs::write(
        config_dir.join("config.toml"),
        r#"
[agent]
default = "claude"
"#,
    )
    .expect("Failed to write valid config");

    // Run kild create command (will fail because not in git repo, but that's fine)
    let output = Command::new(env!("CARGO_BIN_EXE_kild"))
        .current_dir(temp_dir.path())
        .args(["create", "test-branch"])
        .output()
        .expect("Failed to execute kild");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify no config warning is shown (other errors are expected)
    assert!(
        !stderr.contains("Warning: Could not load config"),
        "Unexpected config warning in stderr: {}",
        stderr
    );
}
