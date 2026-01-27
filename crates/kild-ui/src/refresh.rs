//! Background refresh logic for status dashboard.
//!
//! Provides auto-refresh functionality that polls process status
//! every 5 seconds without full session reload.

use std::time::Duration;

/// Refresh interval for auto-update (5 seconds as per PRD)
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(5);
