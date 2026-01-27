use crate::health::types::{HealthMetrics, HealthOutput, HealthStatus, KildHealth};
use crate::process::types::ProcessMetrics;
use crate::sessions::types::Session;
use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicU64, Ordering};

static IDLE_THRESHOLD_MINUTES: AtomicU64 = AtomicU64::new(10);

/// Set the idle threshold for health status calculation
pub fn set_idle_threshold_minutes(minutes: u64) {
    IDLE_THRESHOLD_MINUTES.store(minutes, Ordering::Relaxed);
}

/// Get the current idle threshold
pub fn get_idle_threshold_minutes() -> u64 {
    IDLE_THRESHOLD_MINUTES.load(Ordering::Relaxed)
}

/// Calculate health status based on process state and activity
pub fn calculate_health_status(
    process_running: bool,
    last_activity: Option<&str>,
    last_message_from_user: bool,
) -> HealthStatus {
    if !process_running {
        return HealthStatus::Crashed;
    }

    let Some(activity_str) = last_activity else {
        return HealthStatus::Unknown;
    };

    let Ok(activity_time) = DateTime::parse_from_rfc3339(activity_str) else {
        return HealthStatus::Unknown;
    };

    let now = Utc::now();
    let minutes_since_activity = (now.signed_duration_since(activity_time)).num_minutes();
    let threshold = IDLE_THRESHOLD_MINUTES.load(Ordering::Relaxed);

    // Compare as i64 (threshold fits in i64, and minutes_since_activity is i64)
    if minutes_since_activity < threshold as i64 {
        HealthStatus::Working
    } else if last_message_from_user {
        HealthStatus::Stuck
    } else {
        HealthStatus::Idle
    }
}

/// Enrich session with health metrics
pub fn enrich_session_with_health(
    session: &Session,
    process_metrics: Option<ProcessMetrics>,
    process_running: bool,
) -> KildHealth {
    let status = calculate_health_status(
        process_running,
        session.last_activity.as_deref(),
        false, // TODO: Track last message sender in future
    );

    let status_icon = match status {
        HealthStatus::Working => "✅",
        HealthStatus::Idle => "⏸️ ",
        HealthStatus::Stuck => "⚠️ ",
        HealthStatus::Crashed => "❌",
        HealthStatus::Unknown => "❓",
    };

    let metrics = HealthMetrics {
        cpu_usage_percent: process_metrics.as_ref().map(|m| m.cpu_usage_percent),
        memory_usage_mb: process_metrics.as_ref().map(|m| m.memory_usage_mb()),
        process_status: if process_running {
            "Running".to_string()
        } else {
            "Stopped".to_string()
        },
        last_activity: session.last_activity.clone(),
        status,
        status_icon: status_icon.to_string(),
    };

    KildHealth {
        session_id: session.id.clone(),
        project_id: session.project_id.clone(),
        branch: session.branch.clone(),
        agent: session.agent.clone(),
        worktree_path: session.worktree_path.display().to_string(),
        created_at: session.created_at.clone(),
        metrics,
    }
}

/// Aggregate health statistics
pub fn aggregate_health_stats(kilds: &[KildHealth]) -> HealthOutput {
    let mut working = 0;
    let mut idle = 0;
    let mut stuck = 0;
    let mut crashed = 0;

    for kild in kilds {
        match kild.metrics.status {
            HealthStatus::Working => working += 1,
            HealthStatus::Idle => idle += 1,
            HealthStatus::Stuck => stuck += 1,
            HealthStatus::Crashed => crashed += 1,
            HealthStatus::Unknown => {}
        }
    }

    HealthOutput {
        kilds: kilds.to_vec(),
        total_count: kilds.len(),
        working_count: working,
        idle_count: idle,
        stuck_count: stuck,
        crashed_count: crashed,
    }
}
