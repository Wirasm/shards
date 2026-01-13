use sysinfo::{System, Pid, ProcessesToUpdate};
use crate::process::errors::ProcessError;

/// Check if a process with the given PID is currently running
pub fn is_process_running(pid: u32) -> Result<bool, ProcessError> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    
    let pid = Pid::from_u32(pid);
    Ok(system.process(pid).is_some())
}

/// Kill a process with the given PID
pub fn kill_process(pid: u32) -> Result<(), ProcessError> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    
    let pid_obj = Pid::from_u32(pid);
    
    match system.process(pid_obj) {
        Some(process) => {
            if process.kill() {
                Ok(())
            } else {
                Err(ProcessError::KillFailed { 
                    pid, 
                    message: "Process kill signal failed".to_string() 
                })
            }
        }
        None => Err(ProcessError::NotFound { pid }),
    }
}

/// Get basic information about a process
pub fn get_process_info(pid: u32) -> Result<ProcessInfo, ProcessError> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    
    let pid_obj = Pid::from_u32(pid);
    
    match system.process(pid_obj) {
        Some(process) => {
            Ok(ProcessInfo {
                pid,
                name: process.name().to_string_lossy().to_string(),
                status: if process.status().to_string().is_empty() { 
                    "Running".to_string() 
                } else { 
                    process.status().to_string() 
                },
            })
        }
        None => Err(ProcessError::NotFound { pid }),
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn test_is_process_running_with_invalid_pid() {
        // Use a very high PID that's unlikely to exist
        let result = is_process_running(999999);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_get_process_info_with_invalid_pid() {
        let result = get_process_info(999999);
        assert!(matches!(result, Err(ProcessError::NotFound { pid: 999999 })));
    }

    #[test]
    fn test_kill_process_with_invalid_pid() {
        let result = kill_process(999999);
        assert!(matches!(result, Err(ProcessError::NotFound { pid: 999999 })));
    }

    #[test]
    fn test_process_lifecycle() {
        // Spawn a long-running process
        let mut child = Command::new("sleep")
            .arg("10")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let pid = child.id();

        // Test that process is running
        let is_running = is_process_running(pid).expect("Failed to check process");
        assert!(is_running);

        // Test getting process info
        let info = get_process_info(pid).expect("Failed to get process info");
        assert_eq!(info.pid, pid);
        assert!(info.name.contains("sleep"));

        // Test killing the process
        let kill_result = kill_process(pid);
        assert!(kill_result.is_ok());

        // Clean up - ensure child is terminated
        let _ = child.kill();
        let _ = child.wait();
    }
}
