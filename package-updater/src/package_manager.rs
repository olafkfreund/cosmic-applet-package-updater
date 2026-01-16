use anyhow::{anyhow, Result};
use nix::fcntl::{flock, FlockArg};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::Command;
use tokio::process::Command as TokioCommand;

// Retry and timing constants
const LOCK_RETRY_DELAY_SECS: u64 = 2;
const UPDATE_RETRY_DELAY_SECS: u64 = 1;

// Compiled regex patterns for NixOS flake parsing
static FLAKE_UPDATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:Updated|updated|updating|Will update)\s+(?:input\s+)?['"]?([^\s':]+)['"]?:?\s+['"]?([^'"]+)['"]?\s+(?:->|→|to)\s+['"]?([^'"]+)['"]?"#).unwrap()
});

/// Package manager types supported by the updater applet.
///
/// Each variant represents a different Linux package manager or distribution
/// package management system. The enum is marked `#[non_exhaustive]` to allow
/// adding new package managers without breaking existing code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PackageManager {
    // Arch Linux
    Pacman,
    Paru,
    Yay,
    // Debian/Ubuntu
    Apt,
    // Fedora/RHEL
    Dnf,
    // openSUSE/SUSE
    Zypper,
    // Alpine Linux
    Apk,
    // Universal
    Flatpak,
    // NixOS
    NixOS,
}

impl PackageManager {
    pub fn name(&self) -> &'static str {
        match self {
            PackageManager::Pacman => "pacman",
            PackageManager::Paru => "paru",
            PackageManager::Yay => "yay",
            PackageManager::Apt => "apt",
            PackageManager::Dnf => "dnf",
            PackageManager::Zypper => "zypper",
            PackageManager::Apk => "apk",
            PackageManager::Flatpak => "flatpak",
            PackageManager::NixOS => "nixos",
        }
    }

    pub fn supports_aur(&self) -> bool {
        matches!(self, PackageManager::Paru | PackageManager::Yay)
    }

    /// Get the system update command for this package manager.
    ///
    /// # Arguments
    ///
    /// * `nixos_config` - NixOS configuration (required for NixOS package manager)
    pub fn system_update_command(
        &self,
        nixos_config: Option<&crate::config::NixOSConfig>,
    ) -> String {
        match self {
            PackageManager::Pacman => "sudo pacman -Syu".to_string(),
            PackageManager::Paru => "paru -Syu".to_string(),
            PackageManager::Yay => "yay -Syu".to_string(),
            PackageManager::Apt => "sudo apt update && sudo apt upgrade".to_string(),
            PackageManager::Dnf => "sudo dnf upgrade".to_string(),
            PackageManager::Zypper => "sudo zypper update".to_string(),
            PackageManager::Apk => "sudo apk upgrade".to_string(),
            PackageManager::Flatpak => "flatpak update".to_string(),
            PackageManager::NixOS => {
                if let Some(config) = nixos_config {
                    match config.mode {
                        crate::config::NixOSMode::Channels => {
                            "sudo nix-channel --update && sudo nixos-rebuild switch --upgrade"
                                .to_string()
                        }
                        crate::config::NixOSMode::Flakes => {
                            format!(
                                "cd {} && nix flake update && sudo nixos-rebuild switch --flake .#",
                                config.config_path
                            )
                        }
                    }
                } else {
                    "sudo nixos-rebuild switch".to_string()
                }
            }
        }
    }
}

impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Summary of available package updates.
///
/// Contains counts of updates by type and a list of individual package updates.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// Total number of updates available
    pub total_updates: usize,
    /// Number of official repository updates
    pub official_updates: usize,
    /// Number of AUR (Arch User Repository) updates
    pub aur_updates: usize,
    /// Detailed list of package updates
    pub packages: Vec<PackageUpdate>,
}

/// Information about a single package update.
///
/// Represents an available update for one package, including version information
/// and whether it's from the AUR (for Arch-based systems).
#[derive(Debug, Clone)]
pub struct PackageUpdate {
    /// Package name
    pub name: String,
    /// Currently installed version (may be "unknown")
    pub current_version: String,
    /// New version available
    pub new_version: String,
    /// Whether this is an AUR package (Arch Linux only)
    pub is_aur: bool,
}

impl UpdateInfo {
    pub fn new() -> Self {
        Self {
            total_updates: 0,
            official_updates: 0,
            aur_updates: 0,
            packages: Vec::new(),
        }
    }

    pub fn has_updates(&self) -> bool {
        self.total_updates > 0
    }
}

/// Detects which package managers are available on the system.
///
/// Scans the system to find installed package managers and provides
/// utilities for selecting the preferred one.
pub struct PackageManagerDetector;

impl PackageManagerDetector {
    pub fn detect_available() -> Vec<PackageManager> {
        let mut available = Vec::new();

        // Check in order of preference
        for pm in [
            // AUR helpers first (most feature-rich for Arch)
            PackageManager::Paru,
            PackageManager::Yay,
            // System package managers
            PackageManager::Pacman,
            PackageManager::Apt,
            PackageManager::Dnf,
            PackageManager::Zypper,
            PackageManager::Apk,
            // NixOS
            PackageManager::NixOS,
            // Universal package managers
            PackageManager::Flatpak,
        ] {
            if Self::is_available(pm) {
                available.push(pm);
            }
        }

        available
    }

    pub fn get_preferred() -> Option<PackageManager> {
        Self::detect_available().into_iter().next()
    }

    fn is_available(pm: PackageManager) -> bool {
        match pm {
            PackageManager::NixOS => Self::is_nixos_available(),
            _ => {
                if let Ok(output) = Command::new("which").arg(pm.name()).output() {
                    if output.status.success() {
                        let path = String::from_utf8_lossy(&output.stdout);
                        let path = path.trim();

                        // Verify it's in a system path (not in /tmp, home dir, etc.)
                        // This prevents executing arbitrary binaries from unsafe locations
                        path.starts_with("/usr/")
                            || path.starts_with("/bin/")
                            || path.starts_with("/sbin/")
                            || path.starts_with("/nix/store/")
                            || path.starts_with("/run/current-system/")
                            || path.starts_with("/opt/")
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }

    fn is_nixos_available() -> bool {
        // Check if nixos-rebuild exists
        let nixos_rebuild = Command::new("which")
            .arg("nixos-rebuild")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !nixos_rebuild {
            return false;
        }

        // Check if we're actually on NixOS
        std::path::Path::new("/etc/NIXOS").exists()
            || std::path::Path::new("/run/current-system").exists()
    }

    pub fn detect_nixos_mode(config_path: &str) -> crate::config::NixOSMode {
        let flake_path = std::path::Path::new(config_path).join("flake.nix");
        if flake_path.exists() {
            crate::config::NixOSMode::Flakes
        } else {
            crate::config::NixOSMode::Channels
        }
    }
}

/// Manages package update checking across multiple Linux distributions.
///
/// Handles the execution of package manager commands, parsing their output,
/// and managing concurrent access through file-based locking.
///
/// # Supported Package Managers
///
/// - **Arch Linux**: pacman, paru, yay (with AUR support)
/// - **Debian/Ubuntu**: apt
/// - **Fedora/RHEL**: dnf
/// - **openSUSE**: zypper
/// - **Alpine**: apk
/// - **NixOS**: channels and flakes modes
/// - **Universal**: flatpak
pub struct UpdateChecker {
    package_manager: PackageManager,
}

impl UpdateChecker {
    /// Create a new update checker for the specified package manager.
    pub fn new(package_manager: PackageManager) -> Self {
        Self { package_manager }
    }

    fn get_lock_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(runtime_dir).join("cosmic-package-updater.lock")
    }

    fn get_sync_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(runtime_dir).join("cosmic-package-updater.sync")
    }

    fn notify_check_completed() {
        // Touch the sync file to notify other instances
        let sync_path = Self::get_sync_path();
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&sync_path)
        {
            if let Err(e) = writeln!(
                file,
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ) {
                eprintln!("Warning: Failed to write sync file: {}", e);
            }
        }
    }

    /// Acquire an exclusive lock using flock to prevent concurrent update checks
    async fn acquire_lock() -> Result<File> {
        let lock_path = Self::get_lock_path();

        // Open or create the lock file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&lock_path)?;

        // Try to acquire an exclusive non-blocking lock
        match flock(file.as_raw_fd(), FlockArg::LockExclusiveNonblock) {
            Ok(()) => {
                // Successfully acquired lock, write our PID
                if let Err(e) = writeln!(&file, "{}", std::process::id()) {
                    eprintln!("Warning: Failed to write PID to lock file: {}", e);
                }
                Ok(file)
            }
            Err(nix::errno::Errno::EWOULDBLOCK) => {
                // Lock is held by another process
                Err(anyhow!("Another instance is checking for updates"))
            }
            Err(e) => Err(anyhow!("Failed to acquire lock: {}", e)),
        }
    }

    /// Check for available updates.
    ///
    /// # Arguments
    ///
    /// * `include_aur` - Whether to check AUR packages (only for paru/yay)
    /// * `nixos_config` - NixOS-specific configuration (mode and config path)
    ///
    /// # Returns
    ///
    /// `UpdateInfo` containing all available updates, or an error if the check failed
    pub async fn check_updates(
        &self,
        include_aur: bool,
        nixos_config: &crate::config::NixOSConfig,
    ) -> Result<UpdateInfo> {
        // Try to acquire lock first
        let _lock = match Self::acquire_lock().await {
            Ok(lock) => lock,
            Err(e) => {
                eprintln!("Could not acquire lock: {}. Waiting and retrying...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(LOCK_RETRY_DELAY_SECS)).await;

                // Retry once
                match Self::acquire_lock().await {
                    Ok(lock) => lock,
                    Err(e) => return Err(anyhow!("Update check already in progress: {}", e)),
                }
            }
        };

        let mut update_info = UpdateInfo::new();

        // Step 1: Check official updates first and wait for completion
        match self.check_official_updates(nixos_config).await {
            Ok(official_updates) => {
                let count = official_updates.len();
                update_info.official_updates = count;
                update_info.packages.extend(official_updates);
            }
            Err(e) => {
                eprintln!("Failed to check official updates: {}", e);
                // Retry once after a delay
                tokio::time::sleep(tokio::time::Duration::from_secs(UPDATE_RETRY_DELAY_SECS)).await;
                match self.check_official_updates(nixos_config).await {
                    Ok(official_updates) => {
                        let count = official_updates.len();
                        update_info.official_updates = count;
                        update_info.packages.extend(official_updates);
                    }
                    Err(e) => {
                        eprintln!("Retry failed for official updates: {}", e);
                        // Continue with AUR check even if official fails
                    }
                }
            }
        }

        // Step 2: Only after official check is done, check AUR updates if enabled
        if include_aur && self.package_manager.supports_aur() {
            match self.check_aur_updates().await {
                Ok(aur_updates) => {
                    let count = aur_updates.len();
                    update_info.aur_updates = count;
                    update_info.packages.extend(aur_updates);
                }
                Err(e) => {
                    eprintln!("Failed to check AUR updates: {}", e);
                    // Retry once after a delay
                    tokio::time::sleep(tokio::time::Duration::from_secs(UPDATE_RETRY_DELAY_SECS))
                        .await;
                    match self.check_aur_updates().await {
                        Ok(aur_updates) => {
                            let count = aur_updates.len();
                            update_info.aur_updates = count;
                            update_info.packages.extend(aur_updates);
                        }
                        Err(e) => {
                            eprintln!("Retry failed for AUR updates: {}", e);
                            // Continue even if AUR check fails
                        }
                    }
                }
            }
        }

        // Step 3: Calculate final total only after both checks are complete
        update_info.total_updates = update_info.packages.len();

        // Notify other instances that we completed a check
        Self::notify_check_completed();

        // Lock is automatically released when _lock is dropped
        Ok(update_info)
    }

    async fn check_official_updates(
        &self,
        nixos_config: &crate::config::NixOSConfig,
    ) -> Result<Vec<PackageUpdate>> {
        let (cmd, args) = match self.package_manager {
            // Arch-based systems
            PackageManager::Pacman | PackageManager::Paru | PackageManager::Yay => {
                ("checkupdates", vec![])
            }
            // Debian/Ubuntu
            PackageManager::Apt => ("apt", vec!["list", "--upgradable"]),
            // Fedora/RHEL
            PackageManager::Dnf => ("dnf", vec!["check-update", "-q"]),
            // openSUSE/SUSE
            PackageManager::Zypper => ("zypper", vec!["list-updates"]),
            // Alpine Linux
            PackageManager::Apk => ("apk", vec!["-u", "list"]),
            // Flatpak
            PackageManager::Flatpak => ("flatpak", vec!["remote-ls", "--updates"]),
            // NixOS
            PackageManager::NixOS => {
                return self.check_nixos_updates(nixos_config).await;
            }
        };

        self.parse_update_output(cmd, args, false).await
    }

    async fn check_aur_updates(&self) -> Result<Vec<PackageUpdate>> {
        let (cmd, args) = match self.package_manager {
            PackageManager::Pacman => return Ok(Vec::new()),
            PackageManager::Paru => ("paru", vec!["-Qu", "--aur"]),
            PackageManager::Yay => ("yay", vec!["-Qu", "--aur"]),
            // Other package managers don't have AUR support
            _ => return Ok(Vec::new()),
        };

        self.parse_update_output(cmd, args, true).await
    }

    async fn parse_update_output(
        &self,
        cmd: &str,
        args: Vec<&str>,
        is_aur: bool,
    ) -> Result<Vec<PackageUpdate>> {
        let output = TokioCommand::new(cmd).args(&args).output().await?;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);

            // Handle exit codes more carefully
            // checkupdates returns 2 when no updates are available
            // paru/yay return 1 when no updates are available
            // dnf returns 100 when updates are available, 0 when no updates
            // apt returns non-zero on error but we check stdout
            if (cmd == "checkupdates" && exit_code == 2)
                || ((cmd == "paru" || cmd == "yay") && exit_code == 1)
                || (cmd == "dnf" && exit_code == 100)
            {
                // No updates available or special success case
                if cmd == "dnf" && exit_code == 100 {
                    // dnf exit code 100 means updates ARE available, continue parsing
                } else {
                    return Ok(Vec::new());
                }
            } else {
                // Any other exit code might still have valid output for some package managers
                // Check if we have stdout output before failing
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim().is_empty() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!(
                        "Update check failed with exit code {}: {}",
                        exit_code, stderr
                    );
                    return Err(anyhow!(
                        "Failed to check for updates (exit {}): {}",
                        exit_code,
                        stderr
                    ));
                }
                // Otherwise continue to parse the output
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if let Some(package) = self.parse_package_line(line, is_aur) {
                packages.push(package);
            }
        }

        Ok(packages)
    }

    fn parse_package_line(&self, line: &str, is_aur: bool) -> Option<PackageUpdate> {
        // Skip header lines
        if line.starts_with("Listing...")
            || line.starts_with("Done")
            || line.starts_with("WARNING:")
            || line.starts_with("S |")
            || line.starts_with("--+")
            || line.trim().is_empty()
        {
            return None;
        }

        match self.package_manager {
            // Arch-based: "package 1.0.0-1 -> 1.0.1-1" or "package 1.0.1-1"
            PackageManager::Pacman | PackageManager::Paru | PackageManager::Yay => {
                if line.contains(" -> ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 && parts[2] == "->" {
                        return Some(PackageUpdate {
                            name: parts[0].to_string(),
                            current_version: parts[1].to_string(),
                            new_version: parts[3].to_string(),
                            is_aur,
                        });
                    }
                } else {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        return Some(PackageUpdate {
                            name: parts[0].to_string(),
                            current_version: "unknown".to_string(),
                            new_version: parts[1].to_string(),
                            is_aur,
                        });
                    }
                }
            }

            // APT: "package/suite version arch [upgradable from: old-version]"
            PackageManager::Apt => {
                if line.contains("[upgradable from:") {
                    // Split by '/' to get package name
                    let name = line.split('/').next()?.to_string();

                    // Extract new version (between '/' and architecture)
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let new_version = if parts.len() >= 2 {
                        parts[1].to_string()
                    } else {
                        "unknown".to_string()
                    };

                    // Extract old version from [upgradable from: X]
                    let current_version = if let Some(from_idx) = line.find("[upgradable from: ") {
                        let start = from_idx + "[upgradable from: ".len();
                        if let Some(end_idx) = line[start..].find(']') {
                            line[start..start + end_idx].to_string()
                        } else {
                            "unknown".to_string()
                        }
                    } else {
                        "unknown".to_string()
                    };

                    return Some(PackageUpdate {
                        name,
                        current_version,
                        new_version,
                        is_aur: false,
                    });
                }
            }

            // DNF: "package.arch version repo" (3 columns)
            PackageManager::Dnf => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    // First part is "package.arch"
                    let name = parts[0].split('.').next()?.to_string();
                    let new_version = parts[1].to_string();

                    return Some(PackageUpdate {
                        name,
                        current_version: "unknown".to_string(),
                        new_version,
                        is_aur: false,
                    });
                }
            }

            // Zypper: table format with columns
            // Skip status column and parse name and version
            PackageManager::Zypper => {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 4 {
                    let name = parts[1].trim().to_string();
                    let new_version = parts[3].trim().to_string();

                    return Some(PackageUpdate {
                        name,
                        current_version: "unknown".to_string(),
                        new_version,
                        is_aur: false,
                    });
                }
            }

            // APK: "package-version [upgradable from: old-version]"
            PackageManager::Apk => {
                if line.contains("[upgradable from:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 1 {
                        // First part contains package-version, need to extract package name
                        let pkg_info = parts[0];
                        let name = if let Some(dash_idx) = pkg_info.rfind('-') {
                            pkg_info[..dash_idx].to_string()
                        } else {
                            pkg_info.to_string()
                        };

                        // Extract versions
                        let new_version = parts.get(1).unwrap_or(&"unknown").to_string();

                        let current_version =
                            if let Some(from_idx) = line.find("[upgradable from: ") {
                                let start = from_idx + "[upgradable from: ".len();
                                if let Some(end_idx) = line[start..].find(']') {
                                    line[start..start + end_idx].to_string()
                                } else {
                                    "unknown".to_string()
                                }
                            } else {
                                "unknown".to_string()
                            };

                        return Some(PackageUpdate {
                            name,
                            current_version,
                            new_version,
                            is_aur: false,
                        });
                    }
                }
            }

            // Flatpak: "name\tapp-id\tversion\tbranch\tremote"
            PackageManager::Flatpak => {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    let name = parts[0].to_string();
                    let new_version = parts[2].to_string();

                    return Some(PackageUpdate {
                        name,
                        current_version: "unknown".to_string(),
                        new_version,
                        is_aur: false,
                    });
                }
            }

            // NixOS: Handled separately by check_nixos_updates, never reaches this function
            PackageManager::NixOS => {
                return None;
            }
        }

        None
    }

    /// Check if passwordless sudo is configured for the current user
    async fn check_passwordless_sudo() -> Result<bool> {
        let output = TokioCommand::new("sudo")
            .args(&["-n", "true"]) // -n = non-interactive
            .output()
            .await?;
        Ok(output.status.success())
    }

    async fn check_nixos_channels(&self) -> Result<Vec<PackageUpdate>> {
        // Try PolicyKit first, fall back to passwordless sudo check if not available
        if crate::polkit::PolkitAuth::is_available().await {
            // Use PolicyKit for privilege escalation
            match crate::polkit::execute_privileged(
                "nixos-rebuild",
                &["dry-build", "--upgrade"],
                crate::polkit::POLKIT_ACTION_CHECK,
                "Authentication required to check for NixOS updates",
            )
            .await
            {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Err(anyhow!("Failed to check NixOS updates: {}", stderr));
                    }

                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let combined_output = format!("{}\n{}", stdout, stderr);
                    return self.parse_nixos_rebuild_output(&combined_output);
                }
                Err(e) => {
                    eprintln!("PolicyKit execution failed: {}, trying sudo fallback", e);
                    // Continue to sudo fallback below
                }
            }
        }

        // Fallback to sudo if PolicyKit unavailable or failed
        // Check for passwordless sudo first
        if !Self::check_passwordless_sudo().await.unwrap_or(false) {
            return Err(anyhow!(
                "NixOS channels mode requires passwordless sudo or PolicyKit.\n\
                 \n\
                 Option 1 (Recommended): PolicyKit is not available or failed.\n\
                 Install PolicyKit and ensure pkexec is available.\n\
                 \n\
                 Option 2: Configure passwordless sudo by adding to /etc/sudoers.d/nixos-rebuild:\n\
                 %wheel ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild"
            ));
        }

        // Run nixos-rebuild dry-build with upgrade flag to show package statistics
        let output = TokioCommand::new("sudo")
            .args(&["nixos-rebuild", "dry-build", "--upgrade"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check if it's a permission issue
            if stderr.contains("not allowed")
                || stderr.contains("password")
                || stderr.contains("sudo")
            {
                return Err(anyhow!(
                    "Permission denied. Use PolicyKit or configure passwordless sudo for nixos-rebuild:\n\
                     Add to /etc/sudoers.d/nixos-rebuild:\n\
                     %wheel ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild"
                ));
            }

            return Err(anyhow!("Failed to check NixOS updates: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Combine stdout and stderr as nixos-rebuild outputs to both
        let combined_output = format!("{}\n{}", stdout, stderr);

        // Parse output to detect changes
        self.parse_nixos_rebuild_output(&combined_output)
    }

    /// Parse flake update output to extract input changes
    fn parse_flake_updates(&self, output: &str) -> Vec<PackageUpdate> {
        let mut updates = Vec::new();

        for cap in FLAKE_UPDATE_REGEX.captures_iter(output) {
            if let (Some(input_name), Some(old_ref), Some(new_ref)) =
                (cap.get(1), cap.get(2), cap.get(3))
            {
                let input_name = input_name.as_str();
                let old_ref_str = old_ref.as_str();
                let new_ref_str = new_ref.as_str();

                // Extract commit hashes if present (first 7 chars)
                let old_version = Self::extract_commit_hash(old_ref_str);
                let new_version = Self::extract_commit_hash(new_ref_str);

                updates.push(PackageUpdate {
                    name: format!("flake:{}", input_name),
                    current_version: old_version,
                    new_version,
                    is_aur: false,
                });
            }
        }

        updates
    }

    /// Extract commit hash from git reference (first 7 chars)
    fn extract_commit_hash(git_ref: &str) -> String {
        // Try to extract hash from various formats:
        // - github:NixOS/nixpkgs/abc123def456...
        // - abc123def456789012345678901234567890
        if let Some(hash_start) = git_ref.rfind('/') {
            let hash = &git_ref[hash_start + 1..];
            if hash.len() >= 7 {
                return hash[..7].to_string();
            }
        }

        // If it's already a hash
        if git_ref.len() >= 7 && git_ref.chars().all(|c| c.is_ascii_hexdigit()) {
            return git_ref[..7].to_string();
        }

        // Fallback: use the whole string (truncated if necessary)
        if git_ref.len() > 12 {
            git_ref[..12].to_string()
        } else {
            git_ref.to_string()
        }
    }

    async fn check_nixos_flakes(&self, config_path: &str) -> Result<Vec<PackageUpdate>> {
        let flake_lock_path = std::path::Path::new(config_path).join("flake.lock");

        // Check if flake.lock exists
        if !flake_lock_path.exists() {
            return Err(anyhow!(
                "flake.lock not found in {}. Run 'nix flake update' first.",
                config_path
            ));
        }

        let mut all_updates = Vec::new();

        // First, check for flake input updates using nix flake metadata
        let metadata_output = TokioCommand::new("nix")
            .args(&["flake", "metadata", "--json", config_path])
            .output()
            .await;

        // Check what updates are available (dry-run)
        let update_check = TokioCommand::new("nix")
            .args(&["flake", "update", "--dry-run", config_path])
            .output()
            .await;

        if let Ok(output) = update_check {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{}\n{}", stdout, stderr);

            // Check if already up to date
            if combined.contains("up to date") || combined.contains("no updates") {
                return Ok(Vec::new());
            }

            let flake_updates = self.parse_flake_updates(&combined);
            all_updates.extend(flake_updates);
        }

        // If we found flake input updates, also check what would be rebuilt
        if !all_updates.is_empty() {
            let rebuild_output = TokioCommand::new("nixos-rebuild")
                .args(&["dry-build", "--flake", &format!("{}#", config_path)])
                .output()
                .await;

            if let Ok(output) = rebuild_output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined_output = format!("{}\n{}", stdout, stderr);

                if let Ok(rebuild_updates) = self.parse_nixos_rebuild_output(&combined_output) {
                    all_updates.extend(rebuild_updates);
                }
            }
        }

        Ok(all_updates)
    }

    fn parse_nixos_rebuild_output(&self, output: &str) -> Result<Vec<PackageUpdate>> {
        let mut updates = Vec::new();

        // Count packages that will be built
        // Look for lines like: "these X derivations will be built:"
        let mut packages_to_build = 0;
        let mut packages_to_fetch = 0;

        for line in output.lines() {
            // Match: "these 47 derivations will be built:"
            if line.contains("derivations will be built")
                || line.contains("derivation will be built")
            {
                if let Some(num_str) = line.split_whitespace().nth(1) {
                    if let Ok(num) = num_str.parse::<usize>() {
                        packages_to_build = num;
                    }
                }
            }
            // Match: "these 23 paths will be fetched"
            if line.contains("paths will be fetched") || line.contains("path will be fetched") {
                if let Some(num_str) = line.split_whitespace().nth(1) {
                    if let Ok(num) = num_str.parse::<usize>() {
                        packages_to_fetch = num;
                    }
                }
            }
        }

        // If we found updates, create summary entries
        if packages_to_build > 0 || packages_to_fetch > 0 {
            if packages_to_build > 0 {
                updates.push(PackageUpdate {
                    name: "System Update".to_string(),
                    current_version: format!("{} packages", packages_to_build),
                    new_version: "will be built".to_string(),
                    is_aur: false,
                });
            }

            if packages_to_fetch > 0 {
                updates.push(PackageUpdate {
                    name: "Downloads".to_string(),
                    current_version: format!("{} packages", packages_to_fetch),
                    new_version: "will be fetched".to_string(),
                    is_aur: false,
                });
            }
        } else {
            // Check if system is up to date
            if output.contains("up to date") || output.contains("already built") {
                return Ok(Vec::new());
            }
        }

        Ok(updates)
    }

    async fn check_nixos_updates(
        &self,
        config: &crate::config::NixOSConfig,
    ) -> Result<Vec<PackageUpdate>> {
        match config.mode {
            crate::config::NixOSMode::Channels => self.check_nixos_channels().await,
            crate::config::NixOSMode::Flakes => self.check_nixos_flakes(&config.config_path).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arch_package_line_with_arrow() {
        let checker = UpdateChecker::new(PackageManager::Pacman);
        let line = "linux 6.1.0-1 -> 6.2.0-1";
        let update = checker.parse_package_line(line, false).unwrap();

        assert_eq!(update.name, "linux");
        assert_eq!(update.current_version, "6.1.0-1");
        assert_eq!(update.new_version, "6.2.0-1");
        assert!(!update.is_aur);
    }

    #[test]
    fn test_parse_arch_package_line_without_arrow() {
        let checker = UpdateChecker::new(PackageManager::Pacman);
        let line = "firefox 120.0-1";
        let update = checker.parse_package_line(line, false).unwrap();

        assert_eq!(update.name, "firefox");
        assert_eq!(update.current_version, "unknown");
        assert_eq!(update.new_version, "120.0-1");
        assert!(!update.is_aur);
    }

    #[test]
    fn test_parse_aur_package_line() {
        let checker = UpdateChecker::new(PackageManager::Paru);
        let line = "yay-bin 12.0.0-1 -> 12.1.0-1";
        let update = checker.parse_package_line(line, true).unwrap();

        assert_eq!(update.name, "yay-bin");
        assert_eq!(update.current_version, "12.0.0-1");
        assert_eq!(update.new_version, "12.1.0-1");
        assert!(update.is_aur);
    }

    #[test]
    fn test_parse_apt_package_line() {
        let checker = UpdateChecker::new(PackageManager::Apt);
        let line = "firefox/jammy-updates 120.0+build1-0ubuntu0.22.04.1 amd64 [upgradable from: 119.0+build2-0ubuntu0.22.04.1]";
        let update = checker.parse_package_line(line, false).unwrap();

        assert_eq!(update.name, "firefox");
        assert_eq!(update.new_version, "120.0+build1-0ubuntu0.22.04.1");
        assert_eq!(update.current_version, "119.0+build2-0ubuntu0.22.04.1");
        assert!(!update.is_aur);
    }

    #[test]
    fn test_parse_dnf_package_line() {
        let checker = UpdateChecker::new(PackageManager::Dnf);
        let line = "kernel.x86_64 6.5.0-1.fc38 updates";
        let update = checker.parse_package_line(line, false).unwrap();

        assert_eq!(update.name, "kernel");
        assert_eq!(update.new_version, "6.5.0-1.fc38");
        assert_eq!(update.current_version, "unknown");
        assert!(!update.is_aur);
    }

    #[test]
    fn test_parse_zypper_package_line() {
        let checker = UpdateChecker::new(PackageManager::Zypper);
        let line = "v | firefox | package | 120.0-1.1 | x86_64";
        let update = checker.parse_package_line(line, false).unwrap();

        assert_eq!(update.name, "firefox");
        assert_eq!(update.new_version, "120.0-1.1");
        assert!(!update.is_aur);
    }

    #[test]
    fn test_parse_flatpak_package_line() {
        let checker = UpdateChecker::new(PackageManager::Flatpak);
        let line = "Firefox\torg.mozilla.firefox\t120.0\tstable\tflathub";
        let update = checker.parse_package_line(line, false).unwrap();

        assert_eq!(update.name, "Firefox");
        assert_eq!(update.new_version, "120.0");
        assert!(!update.is_aur);
    }

    #[test]
    fn test_skip_header_lines() {
        let checker = UpdateChecker::new(PackageManager::Apt);
        let header1 = "Listing...";
        let header2 = "Done";
        let header3 = "WARNING: some warning";

        assert!(checker.parse_package_line(header1, false).is_none());
        assert!(checker.parse_package_line(header2, false).is_none());
        assert!(checker.parse_package_line(header3, false).is_none());
    }

    #[test]
    fn test_parse_nixos_rebuild_output() {
        let checker = UpdateChecker::new(PackageManager::NixOS);
        let output = "these 47 derivations will be built:\n  /nix/store/abc...\nthese 23 paths will be fetched (15.2 MiB download, 89.3 MiB unpacked):";
        let updates = checker.parse_nixos_rebuild_output(output).unwrap();

        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].name, "System Update");
        assert!(updates[0].current_version.contains("47"));
        assert_eq!(updates[1].name, "Downloads");
        assert!(updates[1].current_version.contains("23"));
    }

    #[test]
    fn test_parse_nixos_up_to_date() {
        let checker = UpdateChecker::new(PackageManager::NixOS);
        let output = "System is up to date";
        let updates = checker.parse_nixos_rebuild_output(output).unwrap();

        assert_eq!(updates.len(), 0);
    }

    #[test]
    fn test_extract_commit_hash_from_github_ref() {
        let git_ref = "github:NixOS/nixpkgs/abc123def456789";
        let hash = UpdateChecker::extract_commit_hash(git_ref);

        assert_eq!(hash, "abc123d");
    }

    #[test]
    fn test_extract_commit_hash_from_plain_hash() {
        let git_ref = "abc123def456789012345678901234567890";
        let hash = UpdateChecker::extract_commit_hash(git_ref);

        assert_eq!(hash, "abc123d");
    }

    #[test]
    fn test_extract_commit_hash_short_string() {
        let git_ref = "v1.2.3";
        let hash = UpdateChecker::extract_commit_hash(git_ref);

        assert_eq!(hash, "v1.2.3");
    }

    #[test]
    fn test_parse_flake_updates() {
        let checker = UpdateChecker::new(PackageManager::NixOS);
        let output = "Updated input 'nixpkgs': 'github:NixOS/nixpkgs/abc123def' -> 'github:NixOS/nixpkgs/def456abc'";
        let updates = checker.parse_flake_updates(output);

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].name, "flake:nixpkgs");
        assert_eq!(updates[0].current_version, "abc123d");
        assert_eq!(updates[0].new_version, "def456a");
    }

    #[test]
    fn test_package_manager_name() {
        assert_eq!(PackageManager::Pacman.name(), "pacman");
        assert_eq!(PackageManager::Paru.name(), "paru");
        assert_eq!(PackageManager::Apt.name(), "apt");
        assert_eq!(PackageManager::Dnf.name(), "dnf");
        assert_eq!(PackageManager::NixOS.name(), "nixos");
    }

    #[test]
    fn test_package_manager_supports_aur() {
        assert!(PackageManager::Paru.supports_aur());
        assert!(PackageManager::Yay.supports_aur());
        assert!(!PackageManager::Pacman.supports_aur());
        assert!(!PackageManager::Apt.supports_aur());
        assert!(!PackageManager::NixOS.supports_aur());
    }

    #[test]
    fn test_update_info_has_updates() {
        let mut info = UpdateInfo::new();
        assert!(!info.has_updates());

        info.total_updates = 5;
        assert!(info.has_updates());
    }

    #[test]
    fn test_nixos_mode_detection() {
        use std::fs;
        use std::io::Write;

        // Create temporary directory
        let temp_dir = std::env::temp_dir().join(format!("nixos-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();

        // Test Channels mode (no flake.nix)
        let mode = PackageManagerDetector::detect_nixos_mode(temp_dir.to_str().unwrap());
        assert_eq!(mode, crate::config::NixOSMode::Channels);

        // Test Flakes mode (with flake.nix)
        let flake_path = temp_dir.join("flake.nix");
        let mut file = fs::File::create(&flake_path).unwrap();
        writeln!(file, "{{}}").unwrap();

        let mode = PackageManagerDetector::detect_nixos_mode(temp_dir.to_str().unwrap());
        assert_eq!(mode, crate::config::NixOSMode::Flakes);

        // Cleanup
        fs::remove_dir_all(temp_dir).unwrap();
    }

    // Integration tests for lock mechanism
    #[tokio::test]
    async fn test_lock_acquisition_and_release() {
        // This test verifies that a lock can be acquired and automatically released
        let lock_result = UpdateChecker::acquire_lock().await;
        assert!(lock_result.is_ok(), "Failed to acquire lock");

        let lock_file = lock_result.unwrap();
        assert!(lock_file.metadata().is_ok(), "Lock file should exist");

        // Lock is automatically released when lock_file is dropped
        drop(lock_file);

        // Should be able to acquire lock again after release
        let second_lock = UpdateChecker::acquire_lock().await;
        assert!(second_lock.is_ok(), "Failed to acquire lock after release");
    }

    #[tokio::test]
    async fn test_concurrent_lock_prevention() {
        // This test verifies that only one instance can hold the lock at a time
        let lock1 = UpdateChecker::acquire_lock().await;
        assert!(lock1.is_ok(), "First lock acquisition should succeed");

        // Try to acquire lock while first one is held
        let lock2 = UpdateChecker::acquire_lock().await;
        assert!(lock2.is_err(), "Second lock acquisition should fail");
        assert!(
            lock2.unwrap_err().to_string().contains("Another instance"),
            "Error should indicate another instance is running"
        );

        // Release first lock
        drop(lock1);

        // Now should be able to acquire
        let lock3 = UpdateChecker::acquire_lock().await;
        assert!(
            lock3.is_ok(),
            "Lock acquisition should succeed after first lock released"
        );
    }

    #[tokio::test]
    async fn test_lock_retry_logic() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use tokio::time::Duration;

        // Acquire lock in this test
        let lock = UpdateChecker::acquire_lock().await.unwrap();

        // Flag to indicate when lock is released
        let lock_released = Arc::new(AtomicBool::new(false));
        let lock_released_clone = lock_released.clone();

        // Spawn task that holds lock briefly then releases
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            drop(lock);
            lock_released_clone.store(true, Ordering::SeqCst);
        });

        // Wait for lock to be released
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert!(
            lock_released.load(Ordering::SeqCst),
            "Lock should be released"
        );

        // Now acquire should succeed
        let new_lock = UpdateChecker::acquire_lock().await;
        assert!(
            new_lock.is_ok(),
            "Lock acquisition should succeed after wait"
        );
    }

    #[tokio::test]
    async fn test_lock_file_contains_pid() {
        use std::io::Read;

        let lock = UpdateChecker::acquire_lock().await.unwrap();
        let lock_path = UpdateChecker::get_lock_path();

        // Read lock file contents
        let mut file = std::fs::File::open(&lock_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        // Should contain current process ID
        let pid = std::process::id().to_string();
        assert!(
            contents.contains(&pid),
            "Lock file should contain process ID"
        );

        drop(lock);
    }

    #[tokio::test]
    async fn test_sync_notification() {
        use std::io::Read;

        // Remove existing sync file if present
        let sync_path = UpdateChecker::get_sync_path();
        let _ = std::fs::remove_file(&sync_path);

        // Notify check completed
        UpdateChecker::notify_check_completed();

        // Verify sync file was created
        assert!(
            sync_path.exists(),
            "Sync file should exist after notification"
        );

        // Read sync file contents
        let mut file = std::fs::File::open(&sync_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        // Should contain a timestamp
        let timestamp: u64 = contents.trim().parse().unwrap();
        assert!(timestamp > 0, "Sync file should contain valid timestamp");

        // Cleanup
        let _ = std::fs::remove_file(&sync_path);
    }

    #[tokio::test]
    async fn test_multiple_sequential_lock_acquisitions() {
        // Test that multiple sequential lock operations work correctly
        for i in 0..5 {
            let lock = UpdateChecker::acquire_lock().await;
            assert!(lock.is_ok(), "Lock acquisition {} should succeed", i);
            drop(lock);
        }
    }

    #[tokio::test]
    async fn test_lock_path_respects_xdg_runtime_dir() {
        let lock_path = UpdateChecker::get_lock_path();
        let path_str = lock_path.to_string_lossy();

        // Should use XDG_RUNTIME_DIR if set, otherwise /tmp
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            assert!(
                path_str.starts_with(&runtime_dir),
                "Lock path should use XDG_RUNTIME_DIR"
            );
        } else {
            assert!(
                path_str.starts_with("/tmp"),
                "Lock path should use /tmp as fallback"
            );
        }

        assert!(
            path_str.ends_with("cosmic-package-updater.lock"),
            "Lock path should end with correct filename"
        );
    }
}
