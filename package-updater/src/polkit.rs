/// PolicyKit integration for secure privilege escalation
///
/// This module provides PolicyKit (polkit) integration for executing privileged
/// package manager operations without requiring passwordless sudo configuration.
///
/// # Security Benefits
///
/// - Fine-grained permission control
/// - User-friendly authentication dialogs
/// - Session-based authorization caching
/// - Audit logging of privileged operations
/// - No need for sudoers configuration

use anyhow::{anyhow, Result};
use zbus::{Connection, zvariant};
use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use tokio::process::Command as TokioCommand;

/// PolicyKit action identifier for package updates
pub const POLKIT_ACTION_UPDATE: &str = "com.github.cosmic-ext.package-updater.update";

/// PolicyKit action identifier for checking updates
pub const POLKIT_ACTION_CHECK: &str = "com.github.cosmic-ext.package-updater.check";

/// PolicyKit authentication helper using D-Bus
pub struct PolkitAuth {
    connection: Connection,
}

impl PolkitAuth {
    /// Create a new PolicyKit authentication helper
    ///
    /// # Errors
    ///
    /// Returns an error if D-Bus connection cannot be established
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await
            .map_err(|e| anyhow!("Failed to connect to system D-Bus: {}", e))?;
        Ok(Self { connection })
    }

    /// Check if the current user is authorized to perform an action
    ///
    /// # Arguments
    ///
    /// * `action_id` - PolicyKit action identifier (e.g., POLKIT_ACTION_UPDATE)
    ///
    /// # Returns
    ///
    /// `true` if authorized, `false` if not authorized or if PolicyKit is unavailable
    pub async fn check_authorization(&self, action_id: &str) -> Result<bool> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.PolicyKit1",
            "/org/freedesktop/PolicyKit1/Authority",
            "org.freedesktop.PolicyKit1.Authority",
        ).await?;

        // Get current process info for subject
        let pid = std::process::id();
        let subject: HashMap<&str, zvariant::Value> = [
            ("pid", zvariant::Value::U32(pid)),
            ("start-time", zvariant::Value::U64(0)),
        ].iter().cloned().collect();

        // Empty details
        let details: HashMap<&str, &str> = HashMap::new();

        // Check authorization
        let result: zbus::Result<(bool, bool, HashMap<String, String>)> = proxy.call(
            "CheckAuthorization",
            &(subject, action_id, details, 1u32, ""),
        ).await;

        match result {
            Ok((is_authorized, _is_challenge, _details)) => Ok(is_authorized),
            Err(e) => {
                eprintln!("PolicyKit authorization check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Request authorization for an action (shows authentication dialog if needed)
    ///
    /// # Arguments
    ///
    /// * `action_id` - PolicyKit action identifier
    /// * `message` - Message to display in authentication dialog
    ///
    /// # Returns
    ///
    /// `true` if authorization was granted, `false` otherwise
    pub async fn request_authorization(&self, action_id: &str, message: &str) -> Result<bool> {
        // First check if already authorized
        if self.check_authorization(action_id).await? {
            return Ok(true);
        }

        // Use pkexec to request authorization interactively
        // This shows the PolicyKit authentication dialog
        let output = TokioCommand::new("pkexec")
            .arg("--user")
            .arg("root")
            .arg("true")  // Just run 'true' to test authorization
            .env("PKEXEC_MESSAGE", message)
            .output()
            .await?;

        Ok(output.status.success())
    }

    /// Execute a command with PolicyKit authorization
    ///
    /// # Arguments
    ///
    /// * `action_id` - PolicyKit action identifier
    /// * `command` - Command to execute
    /// * `args` - Command arguments
    /// * `message` - Message for authentication dialog
    ///
    /// # Returns
    ///
    /// Command output if successful
    pub async fn execute_as_root(
        &self,
        action_id: &str,
        command: &str,
        args: &[&str],
        message: &str,
    ) -> Result<std::process::Output> {
        // Check authorization first
        if !self.check_authorization(action_id).await? {
            // Request authorization
            if !self.request_authorization(action_id, message).await? {
                return Err(anyhow!("Authorization denied"));
            }
        }

        // Execute command with pkexec
        let output = TokioCommand::new("pkexec")
            .arg("--user")
            .arg("root")
            .arg(command)
            .args(args)
            .env("PKEXEC_MESSAGE", message)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute command with pkexec: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Command failed with exit code: {:?}. Stderr: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(output)
    }

    /// Check if PolicyKit is available on the system
    ///
    /// # Returns
    ///
    /// `true` if PolicyKit and pkexec are available
    pub async fn is_available() -> bool {
        // Check if pkexec is available
        let pkexec_check = std::process::Command::new("which")
            .arg("pkexec")
            .output();

        if pkexec_check.is_err() || !pkexec_check.unwrap().status.success() {
            return false;
        }

        // Try to connect to PolicyKit D-Bus service
        if let Ok(conn) = Connection::system().await {
            let proxy_result = zbus::Proxy::new(
                &conn,
                "org.freedesktop.PolicyKit1",
                "/org/freedesktop/PolicyKit1/Authority",
                "org.freedesktop.PolicyKit1.Authority",
            ).await;
            proxy_result.is_ok()
        } else {
            false
        }
    }
}

/// Fallback to sudo if PolicyKit is not available
///
/// This function attempts to use PolicyKit first, and falls back to sudo
/// if PolicyKit is unavailable or authorization fails.
///
/// # Arguments
///
/// * `command` - Command to execute with privilege
/// * `args` - Command arguments
/// * `action_id` - PolicyKit action identifier
/// * `message` - Message for authentication dialog
///
/// # Returns
///
/// Command output if successful
pub async fn execute_privileged(
    command: &str,
    args: &[&str],
    action_id: &str,
    message: &str,
) -> Result<std::process::Output> {
    // Try PolicyKit first
    if PolkitAuth::is_available().await {
        match PolkitAuth::new().await {
            Ok(polkit) => {
                match polkit.execute_as_root(action_id, command, args, message).await {
                    Ok(output) => return Ok(output),
                    Err(e) => {
                        eprintln!("PolicyKit execution failed: {}, falling back to sudo", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to initialize PolicyKit: {}, falling back to sudo", e);
            }
        }
    }

    // Fallback to sudo
    eprintln!("Using sudo fallback for privileged operation");
    let output = TokioCommand::new("sudo")
        .arg(command)
        .args(args)
        .output()
        .await
        .map_err(|e| anyhow!("Failed to execute with sudo: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Command failed with exit code: {:?}",
            output.status.code()
        ));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_polkit_availability_check() {
        // This test just verifies the function doesn't panic
        let available = PolkitAuth::is_available().await;
        println!("PolicyKit available: {}", available);
        // Don't assert true/false as it depends on system configuration
    }

    #[test]
    fn test_action_constants() {
        assert_eq!(POLKIT_ACTION_UPDATE, "com.github.cosmic-ext.package-updater.update");
        assert_eq!(POLKIT_ACTION_CHECK, "com.github.cosmic-ext.package-updater.check");
    }
}
