use clap::{Arg, ArgAction, Command};

pub fn daemon_command() -> Command {
    Command::new("daemon")
        .about("Manage the KILD daemon")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("start")
                .about("Start the KILD daemon in the background")
                .arg(
                    Arg::new("foreground")
                        .long("foreground")
                        .help("Run daemon in the foreground (for debugging)")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(Command::new("stop").about("Stop the running KILD daemon"))
        .subcommand(
            Command::new("status").about("Show daemon status").arg(
                Arg::new("json")
                    .long("json")
                    .help("Output as JSON")
                    .action(ArgAction::SetTrue),
            ),
        )
}

pub fn inject_command() -> Command {
    Command::new("inject")
        .about("Send text to a running daemon worker's stdin")
        .long_about(
            "Writes text to a daemon-managed kild's PTY stdin as the next user \
             prompt turn. Only call when the worker is idle (Stop hook fired).",
        )
        .arg(
            Arg::new("branch")
                .help("Branch name of the target kild")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("text")
                .help("Text to inject (a newline is appended automatically)")
                .required(true)
                .index(2),
        )
}

pub fn attach_command() -> Command {
    Command::new("attach")
        .about("Attach to a daemon-managed kild session")
        .arg(
            Arg::new("branch")
                .help("Branch name of the kild to attach to")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("pane")
                .long("pane")
                .help("Attach to a specific teammate pane (e.g. %1, %2)")
                .value_name("PANE_ID"),
        )
}
