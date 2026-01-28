pub mod backends;
pub mod common;
pub mod errors;
pub mod handler;
pub mod operations;
pub mod registry;
pub mod traits;
pub mod types;

// Re-export commonly used functions for external access
pub use operations::is_terminal_window_open;
