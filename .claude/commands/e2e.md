---
description: Run end-to-end tests for the Shards CLI after merges
allowed-tools: ["Bash", "Read"]
---

# Shards E2E Test Runner

Run a comprehensive end-to-end test of the Shards CLI after merges to main.

## Instructions

Read the E2E testing guide at `.claude/skills/shards/cookbook/e2e-testing.md` and execute all tests in sequence.

**Key points:**
1. Build the release binary first: `cargo build --release --bin shards`
2. Use `./target/release/shards` for all commands (not cargo run)
3. Execute tests in order - if one fails, investigate before continuing
4. Run edge cases after the main sequence
5. Report a summary table at the end

Course-correct if something fails. The goal is a passing test suite, not blind execution.
