use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tracing::{debug, error, info};

use crate::errors::DaemonError;

/// Handle to a live PTY session.
pub struct ManagedPty {
    /// Master end of the PTY. Used for resize and cloning readers.
    master: Box<dyn MasterPty + Send>,
    /// Child process handle. Used for wait/kill.
    child: Box<dyn Child + Send + Sync>,
    /// Writer to PTY stdin. Wrapped in Arc<Mutex<>> because take_writer()
    /// can only be called once, but we need to write from multiple contexts.
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// Current PTY dimensions.
    size: PtySize,
}

impl ManagedPty {
    pub fn size(&self) -> PtySize {
        self.size
    }

    /// Clone the PTY master reader for reading output in a background task.
    pub fn try_clone_reader(&self) -> Result<Box<dyn std::io::Read + Send>, DaemonError> {
        self.master
            .try_clone_reader()
            .map_err(|e| DaemonError::PtyError(format!("clone reader: {}", e)))
    }

    /// Write bytes to PTY stdin.
    pub fn write_stdin(&self, data: &[u8]) -> Result<(), DaemonError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| DaemonError::PtyError(format!("lock writer: {}", e)))?;
        writer
            .write_all(data)
            .map_err(|e| DaemonError::PtyError(format!("write stdin: {}", e)))?;
        writer
            .flush()
            .map_err(|e| DaemonError::PtyError(format!("flush stdin: {}", e)))?;
        Ok(())
    }

    /// Resize the PTY.
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), DaemonError> {
        let new_size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        self.master
            .resize(new_size)
            .map_err(|e| DaemonError::PtyError(format!("resize: {}", e)))?;
        self.size = new_size;
        debug!(
            event = "daemon.pty.resize_completed",
            rows = rows,
            cols = cols,
        );
        Ok(())
    }

    /// Get the child process ID, if available.
    pub fn child_process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// Wait for the child process to exit. Blocks until exit.
    pub fn wait(&mut self) -> Result<portable_pty::ExitStatus, DaemonError> {
        self.child
            .wait()
            .map_err(|e| DaemonError::PtyError(format!("wait: {}", e)))
    }

    /// Kill the child process.
    pub fn kill(&mut self) -> Result<(), DaemonError> {
        self.child
            .kill()
            .map_err(|e| DaemonError::PtyError(format!("kill: {}", e)))
    }
}

/// Manages all live PTY instances in the daemon.
pub struct PtyManager {
    ptys: HashMap<String, ManagedPty>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            ptys: HashMap::new(),
        }
    }

    /// Create a new PTY and spawn a command in it.
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &mut self,
        session_id: &str,
        command: &str,
        args: &[&str],
        working_dir: &std::path::Path,
        rows: u16,
        cols: u16,
        env_vars: &[(String, String)],
    ) -> Result<&ManagedPty, DaemonError> {
        if self.ptys.contains_key(session_id) {
            return Err(DaemonError::SessionAlreadyExists(session_id.to_string()));
        }

        let pty_system = native_pty_system();
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(size)
            .map_err(|e| DaemonError::PtyError(format!("openpty: {}", e)))?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        cmd.cwd(working_dir);

        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        info!(
            event = "daemon.pty.create_started",
            session_id = session_id,
            command = command,
            rows = rows,
            cols = cols,
        );

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| DaemonError::PtyError(format!("spawn: {}", e)))?;

        let pid = child.process_id();

        // Take the writer once (portable-pty only allows one take_writer call)
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| DaemonError::PtyError(format!("take writer: {}", e)))?;

        let managed = ManagedPty {
            master: pair.master,
            child,
            writer: Arc::new(Mutex::new(writer)),
            size,
        };

        self.ptys.insert(session_id.to_string(), managed);

        info!(
            event = "daemon.pty.create_completed",
            session_id = session_id,
            pid = ?pid,
        );

        Ok(self
            .ptys
            .get(session_id)
            .expect("PTY just inserted must exist"))
    }

    /// Get a reference to a managed PTY.
    pub fn get(&self, session_id: &str) -> Option<&ManagedPty> {
        self.ptys.get(session_id)
    }

    /// Get a mutable reference to a managed PTY.
    pub fn get_mut(&mut self, session_id: &str) -> Option<&mut ManagedPty> {
        self.ptys.get_mut(session_id)
    }

    /// Remove and return a managed PTY.
    pub fn remove(&mut self, session_id: &str) -> Option<ManagedPty> {
        let pty = self.ptys.remove(session_id);
        if pty.is_some() {
            debug!(
                event = "daemon.pty.remove_completed",
                session_id = session_id
            );
        }
        pty
    }

    /// Destroy a PTY by killing the child process and removing it.
    pub fn destroy(&mut self, session_id: &str) -> Result<(), DaemonError> {
        if let Some(mut pty) = self.ptys.remove(session_id) {
            info!(
                event = "daemon.pty.destroy_started",
                session_id = session_id,
            );
            if let Err(e) = pty.kill() {
                error!(
                    event = "daemon.pty.destroy_kill_failed",
                    session_id = session_id,
                    error = %e,
                );
            }
            info!(
                event = "daemon.pty.destroy_completed",
                session_id = session_id,
            );
            Ok(())
        } else {
            Err(DaemonError::SessionNotFound(session_id.to_string()))
        }
    }

    /// Number of active PTYs.
    pub fn count(&self) -> usize {
        self.ptys.len()
    }

    /// All session IDs with active PTYs.
    pub fn session_ids(&self) -> Vec<String> {
        self.ptys.keys().cloned().collect()
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}
