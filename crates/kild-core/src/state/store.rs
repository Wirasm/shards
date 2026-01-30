use super::commands::Command;

/// Trait for dispatching business commands.
///
/// Interfaces (CLI, UI, server) implement this trait to execute
/// commands with their specific needs (UI adds event emission,
/// async handling, etc.).
pub trait Store {
    type Error;
    fn dispatch(&mut self, cmd: Command) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_trait_is_implementable() {
        struct TestStore;
        impl Store for TestStore {
            type Error = String;
            fn dispatch(&mut self, _cmd: Command) -> Result<(), String> {
                Ok(())
            }
        }
        let mut store = TestStore;
        assert!(store.dispatch(Command::RefreshSessions).is_ok());
    }

    #[test]
    fn test_store_impl_can_return_error() {
        struct FailingStore;
        impl Store for FailingStore {
            type Error = String;
            fn dispatch(&mut self, _cmd: Command) -> Result<(), String> {
                Err("not implemented".to_string())
            }
        }
        let mut store = FailingStore;
        assert!(store.dispatch(Command::RefreshSessions).is_err());
    }
}
