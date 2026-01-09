use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tokio::process::Command as TokioCommand;
use std::path::PathBuf;
use std::fs::{File, OpenOptions};
use std::io::{Write, ErrorKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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


    pub fn system_update_command(&self) -> String {
        match self {
            PackageManager::Pacman => "sudo pacman -Syu".to_string(),
            PackageManager::Paru => "paru -Syu".to_string(),
            PackageManager::Yay => "yay -Syu".to_string(),
            PackageManager::Apt => "sudo apt update && sudo apt upgrade".to_string(),
            PackageManager::Dnf => "sudo dnf upgrade".to_string(),
            PackageManager::Zypper => "sudo zypper update".to_string(),
            PackageManager::Apk => "sudo apk upgrade".to_string(),
            PackageManager::Flatpak => "flatpak update".to_string(),
            PackageManager::NixOS => "sudo nixos-rebuild switch".to_string(),
        }
    }
}

impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub total_updates: usize,
    pub official_updates: usize,
    pub aur_updates: usize,
    pub packages: Vec<PackageUpdate>,
}

#[derive(Debug, Clone)]
pub struct PackageUpdate {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
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
            _ => Command::new("which")
                .arg(pm.name())
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false),
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
        std::path::Path::new("/etc/NIXOS").exists() ||
        std::path::Path::new("/run/current-system").exists()
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

pub struct UpdateChecker {
    package_manager: PackageManager,
}

impl UpdateChecker {
    pub fn new(package_manager: PackageManager) -> Self {
        Self { package_manager }
    }

    fn get_lock_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(runtime_dir).join("cosmic-package-updater.lock")
    }

    fn get_sync_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/tmp".to_string());
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
            let _ = writeln!(file, "{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs());
        }
    }

    async fn acquire_lock() -> Result<File> {
        let lock_path = Self::get_lock_path();

        // Try to open the lock file exclusively
        match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                // Write our PID to the lock file
                let _ = writeln!(file, "{}", std::process::id());
                Ok(file)
            }
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                Err(anyhow!("Another instance is checking for updates"))
            }
            Err(e) => Err(anyhow!("Failed to acquire lock: {}", e)),
        }
    }

    pub async fn check_updates(&self, include_aur: bool, nixos_config: &crate::config::NixOSConfig) -> Result<UpdateInfo> {
        // Try to acquire lock first
        let _lock = match Self::acquire_lock().await {
            Ok(lock) => lock,
            Err(e) => {
                eprintln!("Could not acquire lock: {}. Waiting and retrying...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

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
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
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
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
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

    async fn check_official_updates(&self, nixos_config: &crate::config::NixOSConfig) -> Result<Vec<PackageUpdate>> {
        let (cmd, args) = match self.package_manager {
            // Arch-based systems
            PackageManager::Pacman | PackageManager::Paru | PackageManager::Yay => {
                ("checkupdates", vec![])
            }
            // Debian/Ubuntu
            PackageManager::Apt => {
                ("apt", vec!["list", "--upgradable"])
            }
            // Fedora/RHEL
            PackageManager::Dnf => {
                ("dnf", vec!["check-update", "-q"])
            }
            // openSUSE/SUSE
            PackageManager::Zypper => {
                ("zypper", vec!["list-updates"])
            }
            // Alpine Linux
            PackageManager::Apk => {
                ("apk", vec!["-u", "list"])
            }
            // Flatpak
            PackageManager::Flatpak => {
                ("flatpak", vec!["remote-ls", "--updates"])
            }
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

    async fn parse_update_output(&self, cmd: &str, args: Vec<&str>, is_aur: bool) -> Result<Vec<PackageUpdate>> {
        let output = TokioCommand::new(cmd)
            .args(&args)
            .output()
            .await?;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);

            // Handle exit codes more carefully
            // checkupdates returns 2 when no updates are available
            // paru/yay return 1 when no updates are available
            // dnf returns 100 when updates are available, 0 when no updates
            // apt returns non-zero on error but we check stdout
            if (cmd == "checkupdates" && exit_code == 2) ||
               ((cmd == "paru" || cmd == "yay") && exit_code == 1) ||
               (cmd == "dnf" && exit_code == 100) {
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
                    eprintln!("Update check failed with exit code {}: {}", exit_code, stderr);
                    return Err(anyhow!("Failed to check for updates (exit {}): {}", exit_code, stderr));
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
        if line.starts_with("Listing...") || line.starts_with("Done") ||
           line.starts_with("WARNING:") || line.starts_with("S |") ||
           line.starts_with("--+") || line.trim().is_empty() {
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

    async fn check_nixos_channels(&self) -> Result<Vec<PackageUpdate>> {
        // Run nixos-rebuild dry-activate with upgrade flag
        let output = TokioCommand::new("sudo")
            .args(&["nixos-rebuild", "dry-activate", "--upgrade"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check if it's a permission issue
            if stderr.contains("not allowed") || stderr.contains("password") || stderr.contains("sudo") {
                return Err(anyhow!(
                    "Permission denied. Configure passwordless sudo for nixos-rebuild:\n\
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

    fn parse_nixos_rebuild_output(&self, output: &str) -> Result<Vec<PackageUpdate>> {
        let mut updates = Vec::new();
        let mut in_changes_section = false;
        let mut section_name = String::new();

        for line in output.lines() {
            // Detect section headers
            if line.contains("would start the following units:") {
                in_changes_section = true;
                section_name = "start".to_string();
                continue;
            } else if line.contains("would restart the following units:") {
                in_changes_section = true;
                section_name = "restart".to_string();
                continue;
            } else if line.contains("would reload the following units:") {
                in_changes_section = true;
                section_name = "reload".to_string();
                continue;
            } else if line.contains("would stop the following units:") {
                in_changes_section = true;
                section_name = "stop".to_string();
                continue;
            }

            // Empty line or new section ends current section
            if line.trim().is_empty() || line.starts_with("would ") {
                if in_changes_section && !line.starts_with("  ") {
                    in_changes_section = false;
                }
            }

            // Parse unit/service name from indented lines
            if in_changes_section && line.starts_with("  ") {
                let service_name = line.trim();
                updates.push(PackageUpdate {
                    name: service_name.to_string(),
                    current_version: "current".to_string(),
                    new_version: format!("will {}", section_name),
                    is_aur: false,
                });
            }
        }

        // If no specific changes found but command succeeded and mentions building
        if updates.is_empty() {
            if output.contains("building") || output.contains("these") {
                updates.push(PackageUpdate {
                    name: "NixOS System".to_string(),
                    current_version: "current generation".to_string(),
                    new_version: "new generation available".to_string(),
                    is_aur: false,
                });
            }
        }

        Ok(updates)
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

        // Backup current flake.lock
        let backup_path = std::path::Path::new(config_path).join("flake.lock.backup");
        tokio::fs::copy(&flake_lock_path, &backup_path).await?;

        // Run nix flake update to see what would change
        // Note: --dry-run doesn't exist, so we backup/restore instead
        let output = TokioCommand::new("nix")
            .args(&["flake", "update"])
            .current_dir(config_path)
            .output()
            .await;

        // Always restore the backup, even if the command failed
        let restore_result = tokio::fs::copy(&backup_path, &flake_lock_path).await;
        let remove_result = tokio::fs::remove_file(&backup_path).await;

        // Check if restore succeeded
        if let Err(e) = restore_result {
            return Err(anyhow!("Failed to restore flake.lock backup: {}", e));
        }

        // Remove backup file (ignore errors)
        let _ = remove_result;

        // Now check if the update command succeeded
        let output = output?;

        // Parse the update output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Nix flake output goes to stderr for some operations
        let combined_output = format!("{}\n{}", stdout, stderr);

        // Look for patterns indicating updates
        self.parse_flake_update_output(&combined_output)
    }

    fn parse_flake_update_output(&self, output: &str) -> Result<Vec<PackageUpdate>> {
        let mut updates = Vec::new();

        // Pattern 1: • Updated input 'name': (multi-line format)
        // • Updated input 'nur':
        //     'github:...' (date)
        //   → 'github:...' (date)
        let input_regex = regex::Regex::new(r"Updated input '([^']+)':")
            .map_err(|e| anyhow!("Failed to compile input regex: {}", e))?;

        let lines: Vec<&str> = output.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if let Some(captures) = input_regex.captures(lines[i]) {
                let input_name = captures.get(1).map(|m| m.as_str()).unwrap_or("unknown");

                // Look for the old and new refs in the next 2 lines
                if i + 2 < lines.len() {
                    let old_line = lines[i + 1].trim();
                    let new_line = lines[i + 2].trim();

                    // Extract revision hashes from URLs
                    // Format: 'github:owner/repo/HASH?narHash=...' (date)
                    let extract_hash = |line: &str| -> Option<String> {
                        // Remove leading → or • and quotes
                        let cleaned = line.trim_start_matches('→').trim_start_matches('•').trim().trim_matches('\'').trim_matches('"');
                        // Look for pattern: /HASH? or /HASH (
                        if let Some(start) = cleaned.rfind('/') {
                            let after_slash = &cleaned[start+1..];
                            // Find end of hash (before ? or space or ()
                            let hash_end = after_slash.find(|c| c == '?' || c == ' ' || c == '(').unwrap_or(after_slash.len());
                            let hash = &after_slash[..hash_end];
                            if hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
                                return Some(hash[..7.min(hash.len())].to_string());
                            }
                        }
                        None
                    };

                    if let (Some(old_hash), Some(new_hash)) = (extract_hash(old_line), extract_hash(new_line)) {
                        updates.push(PackageUpdate {
                            name: format!("flake:{}", input_name),
                            current_version: old_hash,
                            new_version: new_hash,
                            is_aur: false,
                        });
                    }
                }
                i += 3; // Skip the input line and the two following lines
            } else {
                i += 1;
            }
        }

        // Check for "up to date" message
        if output.contains("up to date") || output.contains("no updates") {
            return Ok(Vec::new());
        }

        // If no updates detected but output isn't empty and doesn't say "up to date"
        if updates.is_empty() && !output.trim().is_empty() {
            // Check if there's mention of updates but we couldn't parse them
            if output.contains("update") || output.contains("flake") {
                updates.push(PackageUpdate {
                    name: "NixOS Flake".to_string(),
                    current_version: "check manually".to_string(),
                    new_version: "updates may be available".to_string(),
                    is_aur: false,
                });
            }
        }

        Ok(updates)
    }

    async fn check_nixos_updates(&self, config: &crate::config::NixOSConfig) -> Result<Vec<PackageUpdate>> {
        match config.mode {
            crate::config::NixOSMode::Channels => self.check_nixos_channels().await,
            crate::config::NixOSMode::Flakes => self.check_nixos_flakes(&config.config_path).await,
        }
    }

}