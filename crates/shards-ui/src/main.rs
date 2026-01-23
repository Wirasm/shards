//! shards-ui: GUI for Shards
//!
//! GPUI-based visual dashboard for shard management.
//! See .claude/PRPs/prds/gpui-native-terminal-ui.prd.md for implementation plan.

// NOTE: GPUI import commented out due to compilation issues with GPUI 0.2.2
// The published version has dependency conflicts (core-graphics version mismatch)
// Uncomment when GPUI becomes stable:
// use gpui as _;

fn main() {
    eprintln!("shards-ui: GPUI scaffolding ready (pending GPUI stability).");
    eprintln!("NOTE: GPUI 0.2.2 has compilation issues - waiting for stable release.");
    eprintln!("See Phase 2 of gpui-native-terminal-ui.prd.md to continue when GPUI is stable.");
    std::process::exit(1);
}
