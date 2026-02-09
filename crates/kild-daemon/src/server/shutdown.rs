use tokio_util::sync::CancellationToken;
use tracing::info;

/// Wait for a shutdown signal (SIGTERM or SIGINT/Ctrl-C).
///
/// When the signal is received, cancels the provided token to notify
/// all tasks to drain gracefully.
pub async fn wait_for_shutdown_signal(token: CancellationToken) {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to register SIGTERM handler");

        tokio::select! {
            _ = ctrl_c => {
                info!(event = "daemon.server.signal_received", signal = "SIGINT");
            }
            _ = sigterm.recv() => {
                info!(event = "daemon.server.signal_received", signal = "SIGTERM");
            }
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
        info!(event = "daemon.server.signal_received", signal = "SIGINT");
    }

    token.cancel();
}
