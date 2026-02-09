use kild_peek_core::init_logging;

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
        drop(e);
        std::process::exit(1);
    }
}
