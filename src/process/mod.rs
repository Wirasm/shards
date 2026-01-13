pub mod errors;
pub mod operations;

pub use errors::ProcessError;
pub use operations::{is_process_running, kill_process, get_process_info};
