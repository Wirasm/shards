use clap::{Arg, ArgAction, Command};

pub fn project_command() -> Command {
    Command::new("project")
        .about("Manage the KILD project registry")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("add")
                .about("Register a git repo in the project registry")
                .arg(
                    Arg::new("path")
                        .help("Path to the git repository")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("name")
                        .long("name")
                        .short('n')
                        .help("Override display name"),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("List all registered projects")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Output as JSON")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a project from the registry")
                .arg(
                    Arg::new("identifier")
                        .help("Path or project ID")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("info")
                .about("Show details for a project")
                .arg(
                    Arg::new("identifier")
                        .help("Path or project ID")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("default")
                .about("Set the default active project")
                .arg(
                    Arg::new("identifier")
                        .help("Path or project ID")
                        .required(true)
                        .index(1),
                ),
        )
}
