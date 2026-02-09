use kild_core::init_logging;

mod app;
mod commands;
mod table;

fn main() {
    let app = app::build_cli();
    let matches = app.get_matches();

    let verbose = matches.get_flag("verbose");
    let quiet = !verbose;
    init_logging(quiet);

    if let Err(e) = commands::run_command(&matches) {
        // Error already printed to user via eprintln! in command handlers.
        // In verbose mode, JSON logs were also emitted.
        // Exit with non-zero code without printing Rust's Debug representation.
        drop(e);
        std::process::exit(1);
    }
}
