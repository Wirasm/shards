use tracing::{error, info};

fn main() {
    kild_core::init_logging(false);
    info!(event = "daemon.start_started");

    let exit_code = match run() {
        Ok(()) => {
            info!(event = "daemon.start_completed");
            0
        }
        Err(e) => {
            error!(event = "daemon.start_failed", error = %e);
            eprintln!("kild-daemon: {}", e);
            1
        }
    };
    std::process::exit(exit_code);
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!(
        "Starting daemon in foreground (PID: {})...",
        std::process::id()
    );

    let config = kild_daemon::load_daemon_config()?;
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { kild_daemon::run_server(config).await })?;
    Ok(())
}
