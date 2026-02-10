mod commands;
mod errors;
mod ipc;
mod parser;
mod state;

use std::env;
use std::path::PathBuf;

fn main() {
    setup_logging();

    let args: Vec<String> = env::args().skip(1).collect();
    let exit_code = match run(&args) {
        Ok(code) => code,
        Err(e) => {
            // Only write to stderr if logging is enabled (Claude Code checks stderr)
            if env::var("KILD_SHIM_LOG").is_ok() {
                tracing::error!(event = "shim.run_failed", error = %e);
            }
            1
        }
    };
    std::process::exit(exit_code);
}

fn run(args: &[String]) -> Result<i32, errors::ShimError> {
    let cmd = parser::parse(args)?;
    commands::execute(cmd)
}

fn setup_logging() {
    let log_setting = env::var("KILD_SHIM_LOG").ok();
    if log_setting.is_none() {
        return;
    }

    let log_path = match log_setting.as_deref() {
        Some("1") | Some("true") => {
            let session_id = env::var("KILD_SHIM_SESSION").unwrap_or_default();
            if session_id.is_empty() {
                return;
            }
            let dir = dirs::home_dir()
                .expect("home directory not found")
                .join(".kild")
                .join("shim")
                .join(&session_id);
            std::fs::create_dir_all(&dir).ok();
            dir.join("shim.log")
        }
        Some(path) => PathBuf::from(path),
        None => return,
    };

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .ok();

    if let Some(file) = file {
        use tracing_subscriber::fmt;
        use tracing_subscriber::prelude::*;

        let layer = fmt::layer()
            .json()
            .with_writer(std::sync::Mutex::new(file))
            .with_target(false);

        tracing_subscriber::registry().with(layer).init();
    }
}
