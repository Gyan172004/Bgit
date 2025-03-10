use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

use super::error::create_hook_error;
use super::process::handle_process_output;
use crate::bgit_error::BGitError;

pub fn execute_hook_util(event_hook_path: &Path, event_name: &str) -> Result<bool, Box<BGitError>> {
    if !event_hook_path.exists() {
        return Ok(true);
    }

    let event_hook_path_str = event_hook_path.to_str().ok_or_else(|| {
        create_hook_error(
            "Invalid path",
            "Path contains invalid characters",
            event_name,
        )
    })?;

    // Check if the file is already executable and make it executable if needed
    let metadata = fs::metadata(event_hook_path).map_err(|e| {
        create_hook_error(
            "Failed to get hook file metadata",
            &e.to_string(),
            event_name,
        )
    })?;

    let mut permissions = metadata.permissions();
    if permissions.mode() & 0o111 == 0 {
        // File is not executable, so make it executable
        permissions.set_mode(permissions.mode() | 0o755); // 0o755 gives rwxr-xr-x permissions
        fs::set_permissions(event_hook_path, permissions).map_err(|e| {
            create_hook_error(
                "Failed to make event hook executable",
                &e.to_string(),
                event_name,
            )
        })?;
    }

    // Spawn the command
    let mut child = Command::new(event_hook_path_str)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| create_hook_error("Failed to run event-hook", &e.to_string(), event_name))?;

    // Handle stdout and stderr
    handle_process_output(&mut child)?;

    // Wait for the command to finish
    let status = child
        .wait()
        .map_err(|e| create_hook_error("Failed to wait on child", &e.to_string(), event_name))?;

    if status.success() {
        Ok(true)
    } else {
        Err(create_hook_error(
            "event-hook failed",
            &format!(
                "Event-hook exited with non-zero status {}\nFile:{}",
                status.code().unwrap_or(-1),
                event_hook_path_str
            ),
            event_name,
        ))
    }
}
