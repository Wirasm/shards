use super::events::Event;
use super::types::Command;

/// Trait for dispatching business commands.
///
/// Decouples command definitions from their execution. Interfaces (CLI, UI)
/// implement this trait to execute commands with their specific needs
/// (e.g., UI adds event emission and async handling, CLI runs synchronously).
///
/// # Semantics
///
/// - **Ordering**: Commands execute in the order received. No implicit batching.
/// - **Idempotency**: Commands are not idempotent (e.g., `CreateKild` fails if
///   the branch already exists). Callers must avoid duplicate dispatches.
/// - **Error handling**: Implementations define their own error type. Errors
///   should distinguish user errors (invalid input) from system errors (IO failure).
/// - **Events**: On success, dispatch returns a `Vec<Event>` describing what
///   changed. Callers can use these to react without polling or disk re-reads.
pub trait Store {
    type Error;
    fn dispatch(&mut self, cmd: Command) -> Result<Vec<Event>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_trait_is_implementable() {
        struct TestStore;
        impl Store for TestStore {
            type Error = String;
            fn dispatch(&mut self, _cmd: Command) -> Result<Vec<Event>, String> {
                Ok(vec![])
            }
        }
        let mut store = TestStore;
        let result = store.dispatch(Command::RefreshSessions);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_store_impl_can_return_error() {
        struct FailingStore;
        impl Store for FailingStore {
            type Error = String;
            fn dispatch(&mut self, _cmd: Command) -> Result<Vec<Event>, String> {
                Err("not implemented".to_string())
            }
        }
        let mut store = FailingStore;
        assert!(store.dispatch(Command::RefreshSessions).is_err());
    }
}
