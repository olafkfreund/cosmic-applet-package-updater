# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A COSMIC desktop environment applet for tracking and managing package updates across multiple Linux distributions. Built with Rust using the libcosmic framework, this applet provides visual indicators, automatic update checking, and seamless integration into the COSMIC panel.

## Build and Development Commands

### Building

```bash
# Development build
just build-debug

# Release build (default)
just build-release

# Quick development cycle (format + run)
just dev
```

### Running and Testing

```bash
# Run with debug logging
just run

# Run with environment variables
cd package-updater && env RUST_LOG=cosmic_tasks=info RUST_BACKTRACE=full cargo run --release

# Run tests (when implemented)
cd package-updater && cargo test
```

### Code Quality

```bash
# Run clippy with pedantic warnings
just check

# Format code
cd package-updater && cargo fmt

# Clean build artifacts
just clean
```

### Installation

```bash
# Build and install system-wide
just build-release
sudo just install

# Uninstall
sudo just uninstall
```

## Architecture

### Module Structure

The codebase is organized into four main modules:

- **`main.rs`**: Entry point that initializes the COSMIC applet
- **`app.rs`**: Core application logic, UI rendering, and message handling using the Elm architecture
- **`package_manager.rs`**: Package manager detection, update checking, and output parsing for multiple distros
- **`config.rs`**: Configuration persistence using cosmic-config

### Key Architectural Patterns

#### Elm-Style Message Passing

The applet uses COSMIC's Elm-inspired architecture with a central `update()` function handling all state changes via `Message` enum variants. UI events trigger messages, which are processed to produce new state and optional `Task`s for async operations.

#### Multi-Instance Synchronization

Multiple applet instances synchronize via file-based locking and file watchers:

- **Lock file** (`$XDG_RUNTIME_DIR/cosmic-package-updater.lock`): Prevents concurrent update checks
- **Sync file** (`$XDG_RUNTIME_DIR/cosmic-package-updater.sync`): Notifies other instances when updates are checked
- File watcher subscription in `subscription()` monitors the sync file and triggers `Message::SyncFileChanged`
- Debouncing prevents rapid repeated checks (10-second minimum between syncs)

#### Async Package Manager Calls

All package manager commands run asynchronously using tokio:

- Commands like `checkupdates`, `apt list`, `dnf check-update` execute via `tokio::process::Command`
- Lock acquisition is async with automatic retry logic
- Terminal launching polls for marker file deletion to detect when user closes terminal

#### Exit Code Handling

Package managers have different exit code conventions:

- `checkupdates`: returns 2 when no updates available (not an error)
- `paru`/`yay`: returns 1 when no updates available
- `dnf`: returns 100 when updates ARE available (inverted logic)
- APT: may return non-zero but still have valid stdout output

The `parse_update_output()` function handles these edge cases explicitly.

### Package Manager Support

The `PackageManager` enum represents supported systems:

- **Arch**: Pacman (official), Paru/Yay (with AUR support)
- **Debian/Ubuntu**: APT
- **Fedora/RHEL**: DNF
- **openSUSE**: Zypper
- **Alpine**: APK
- **NixOS**: Channels and Flakes modes
- **Universal**: Flatpak

Each package manager implements:

- Detection via `which` command in `PackageManagerDetector::is_available()`
- Update check command and argument array
- Output parsing logic in `parse_package_line()` (handles different output formats)
- System update command for terminal execution

### UI Structure

The popup window has two tabs implemented via `PopupTab` enum:

1. **Updates Tab**: Shows update status, package list (scrollable, grouped by official/AUR for Arch), and action buttons
2. **Settings Tab**: Package manager selection, check interval, toggles for auto-check/AUR/notifications/update count, terminal preference

The panel icon dynamically changes based on state:

- `package-x-generic-symbolic`: System up to date
- `software-update-available-symbolic`: Updates available
- `view-refresh-symbolic`: Currently checking
- `dialog-error-symbolic`: Error occurred

When `show_update_count` is enabled, uses a custom button with icon + count badge instead of standard icon button.

## Important Technical Details

### Configuration

Settings persist to `~/.config/cosmic/com.github.cosmic_ext.PackageUpdater/` using cosmic-config. The config struct includes:

- Selected package manager (auto-detected on first run if unset)
- Check interval in minutes (1-1440, default 60)
- Auto-check on startup toggle
- AUR inclusion toggle (only visible for Paru/Yay)
- Notification and update count display preferences
- Preferred terminal (default: cosmic-term)

### Terminal Integration

Terminal updates work via a wrapper script pattern:

1. Creates unique marker file in `$XDG_RUNTIME_DIR`
2. Wraps package manager update command to delete marker on completion
3. Spawns terminal with `-e sh -c "wrapped_command"`
4. Polls for marker file deletion (500ms intervals)
5. Waits 3 seconds for system stabilization after terminal closes
6. Triggers `Message::TerminalFinished` → `Message::CheckForUpdates`

This enables automatic re-checking after updates complete.

### Subscription System

Two subscriptions run when a package manager is configured:

1. **Timer subscription**: Fires every `check_interval_minutes` to trigger automatic checks
2. **File watcher subscription**: Monitors sync file for changes from other instances using the `notify` crate

The `ignore_next_sync` flag prevents spurious checks on startup when the file watcher initializes.

### Error Handling

- Lock acquisition retries once after 2-second delay before failing
- Update check commands retry once after 1-second delay on failure
- Wayland protocol errors after system updates show user-friendly message suggesting applet restart
- Empty stdout/stderr checked before treating non-zero exit codes as errors

## Development Guidelines

### Adding Package Manager Support

1. Add variant to `PackageManager` enum
2. Implement `name()`, `supports_aur()`, and `system_update_command()`
3. Add detection logic to `PackageManagerDetector::detect_available()`
4. Implement command/args in `check_official_updates()` or `check_aur_updates()`
5. Add parsing logic in `parse_package_line()` to handle output format
6. Test with actual package manager output for edge cases

### Clippy Configuration

The workspace is configured with strict lints:

```toml
[workspace.lints.clippy]
todo = "warn"
unwrap_used = "warn"
```

Tests are allowed to use unwrap via `clippy.toml`:

```toml
allow-unwrap-in-tests = true
```

### libcosmic Integration

This applet uses libcosmic with specific features:

- `applet`: COSMIC applet infrastructure
- `tokio`: Async runtime integration
- `wayland`: Wayland protocol support
- `autosize`: Dynamic sizing for custom panel buttons

The applet implements `cosmic::Application` trait with specialized `view()` for panel icon and `view_window()` for popup.

## NixOS Implementation Details

### Architecture

NixOS support is implemented through two distinct modes:

1. **Channels Mode** - Traditional NixOS using `nix-channel` and `nixos-rebuild`
2. **Flakes Mode** - Modern approach using `flake.nix` and `flake.lock`

### Configuration Structure

```rust
pub struct NixOSConfig {
    pub mode: NixOSMode,           // Channels or Flakes
    pub config_path: String,        // Default: "/etc/nixos"
}
```

### Detection Logic

**System Detection:**
- Checks for `nixos-rebuild` command via `which`
- Verifies NixOS markers: `/etc/NIXOS` or `/run/current-system`

**Mode Detection:**
- Checks for `flake.nix` in config path
- If present → Flakes mode
- If absent → Channels mode

### Update Checking

**Channels Mode:**
```bash
sudo nixos-rebuild dry-activate --upgrade
```

- Requires sudo access (passwordless recommended)
- Parses output for systemd unit changes (start, restart, reload, stop)
- Identifies which services/units would be affected by update
- Returns list of `PackageUpdate` with service names

**Flakes Mode:**
```bash
nix flake update --dry-run
```

- Doesn't require sudo (reads flake.lock, runs nix command as user)
- Checks `flake.lock` existence first
- Parses output for flake input updates using regex
- Extracts commit hashes and truncates to 7 characters for display
- Returns list of `PackageUpdate` with format `flake:input_name`

### Parsing Strategies

**Channels Output Parsing:**
- Looks for section headers: "would start the following units:", etc.
- Parses indented lines (starting with two spaces) as service names
- Tracks which action (start/restart/reload/stop) applies
- Falls back to generic "new generation available" if no specifics found

**Flakes Output Parsing:**
- Regex pattern: `(?:Updated|Will update|updating|update)\s+(?:input\s+)?'?([^'\s:]+)'?:?\s+['\"]?([^'\"]+)['\"]?\s+(?:->|→|to)\s+['\"]?([^'\"]+)['\"]?`
- Extracts: input name, old reference, new reference
- Handles various output formats (GitHub URLs, commit hashes, etc.)
- Checks for "up to date" or "no updates" messages

### Permission Handling

**Channels Mode:**
- Needs sudo for `nixos-rebuild`
- Error detection for permission issues:
  - Checks stderr for "not allowed", "password", "sudo"
  - Returns helpful error message with sudoers configuration instructions

**Flakes Mode:**
- No sudo needed for checking updates
- May need sudo for actual system rebuild (handled by terminal update command)

### Terminal Update Commands

**Channels:**
```bash
sudo nix-channel --update && sudo nixos-rebuild switch --upgrade
```

**Flakes:**
```bash
cd <config_path> && nix flake update && sudo nixos-rebuild switch --flake .#
```

### Integration Points

1. **`check_official_updates()`** - Routes NixOS to `check_nixos_updates()`
2. **`check_nixos_updates()`** - Dispatches to channels or flakes checker based on mode
3. **`check_updates()`** - Now accepts `nixos_config` parameter
4. **UI Settings Tab** - Shows NixOS-specific controls when NixOS is selected

### Data Flow

```
User selects NixOS → Auto-detect mode (or manual selection)
                    ↓
Update check triggered → check_updates(include_aur, &nixos_config)
                    ↓
check_official_updates(&nixos_config) → match PackageManager::NixOS
                    ↓
check_nixos_updates(&nixos_config) → match mode (Channels/Flakes)
                    ↓
Execute appropriate command → Parse output → Return Vec<PackageUpdate>
```

### Testing Considerations

- Channels mode requires actual NixOS system with sudo access
- Flakes mode requires valid `flake.nix` and `flake.lock`
- Mock the command execution for unit tests
- Test regex patterns against real nix output samples
- Verify permission error messages are clear and helpful

## Common Pitfalls

- **Don't forget to acquire lock** before checking updates - prevents multiple instances from running package manager commands simultaneously
- **Handle exit codes carefully** - different package managers use different conventions for "no updates"
- **Always parse stdout even on non-zero exit** - some package managers return non-zero but still provide valid output
- **Debounce sync events** - without debouncing, file watcher can trigger rapid repeated checks
- **Release lock automatically** - the lock file handle is stored in `_lock` to ensure it's dropped when function returns
- **NixOS channels need sudo** - ensure clear error messages guide users to configure passwordless sudo
- **NixOS is declarative** - updates are shown as "generation changes" not individual packages
- **Flake.lock must exist** - validate file existence before attempting flakes update check

---

## Security Patterns (Added 2026-01-16)

### Command Injection Prevention

**Always use `shell-escape` for user-influenced paths:**

```rust
use shell_escape::escape;

let marker_file = format!("{}/file-{}.marker", runtime_dir, pid);
let escaped_marker = shell_escape::escape(marker_file.into());

// Safe: marker file path is properly escaped
let command = format!(
    "{} && echo 'Done' && read; rm -f {}",
    update_command,
    escaped_marker
);
```

**❌ Don't do:**
```rust
// UNSAFE: No escaping, vulnerable to injection
let command = format!(
    "{} && echo \"Done\" && read; rm -f \"{}\"",
    update_command,
    marker_file  // ⚠️ Not escaped!
);
```

### File Locking Best Practices

**Use atomic `flock` for inter-process coordination:**

```rust
use nix::fcntl::{flock, FlockArg};
use std::os::unix::io::AsRawFd;

async fn acquire_lock() -> Result<File> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&lock_path)?;

    // Atomic non-blocking exclusive lock
    match flock(file.as_raw_fd(), FlockArg::LockExclusiveNonblock) {
        Ok(()) => Ok(file),
        Err(nix::errno::Errno::EWOULDBLOCK) => {
            Err(anyhow!("Another instance is running"))
        }
        Err(e) => Err(anyhow!("Lock failed: {}", e))
    }
}
```

**❌ Don't do:**
```rust
// UNSAFE: Race condition - not atomic!
match OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)  // ⚠️ Multiple processes can pass this check
    .open(&lock_path)
{
    Ok(file) => { /* Lock "acquired" */ }
    Err(_) => { /* Someone else has it? */ }
}
```

### Pre-flight Sudo Checks

**Test for passwordless sudo before attempting privileged operations:**

```rust
async fn check_passwordless_sudo() -> Result<bool> {
    let output = TokioCommand::new("sudo")
        .args(&["-n", "true"])  // -n = non-interactive
        .output()
        .await?;
    Ok(output.status.success())
}

async fn run_privileged_command() -> Result<()> {
    if !check_passwordless_sudo().await? {
        return Err(anyhow!(
            "Passwordless sudo required. Configure:\n\
             %wheel ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild"
        ));
    }
    // Safe to proceed
}
```

### Path Validation for Executables

**Only execute binaries from trusted system directories:**

```rust
fn is_safe_executable_path(path: &str) -> bool {
    path.starts_with("/usr/") ||
    path.starts_with("/bin/") ||
    path.starts_with("/sbin/") ||
    path.starts_with("/nix/store/") ||
    path.starts_with("/run/current-system/") ||
    path.starts_with("/opt/")
}

fn validate_package_manager(pm_name: &str) -> Result<String> {
    let output = Command::new("which").arg(pm_name).output()?;
    if !output.status.success() {
        return Err(anyhow!("Package manager not found"));
    }

    let path = String::from_utf8_lossy(&output.stdout);
    let path = path.trim();

    if !is_safe_executable_path(path) {
        return Err(anyhow!(
            "Package manager in unsafe location: {}",
            path
        ));
    }

    Ok(path.to_string())
}
```

### Error Handling Patterns

**Always log errors, even when using fallbacks:**

```rust
// ✅ Good: Error is logged before fallback
let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
    .unwrap_or_else(|e| {
        eprintln!("Warning: XDG_RUNTIME_DIR not set: {}. Using /tmp", e);
        "/tmp".to_string()
    });

// ✅ Good: Error is reported
if let Err(e) = std::fs::File::create(&marker_file) {
    eprintln!("Warning: Failed to create marker file: {}", e);
}

// ❌ Bad: Silent failure
let _ = std::fs::File::create(&marker_file);
```

---

## Testing Patterns (Added 2026-01-16)

### Unit Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_line_with_version() {
        let checker = UpdateChecker::new(PackageManager::Pacman);
        let line = "linux 6.1.0-1 -> 6.2.0-1";
        let update = checker.parse_package_line(line, false).unwrap();

        assert_eq!(update.name, "linux");
        assert_eq!(update.current_version, "6.1.0-1");
        assert_eq!(update.new_version, "6.2.0-1");
        assert!(!update.is_aur);
    }

    #[test]
    fn test_parse_package_line_without_version() {
        let checker = UpdateChecker::new(PackageManager::Pacman);
        let line = "firefox 120.0-1";
        let update = checker.parse_package_line(line, false).unwrap();

        assert_eq!(update.name, "firefox");
        assert_eq!(update.current_version, "unknown");
        assert_eq!(update.new_version, "120.0-1");
    }

    #[test]
    fn test_skip_header_lines() {
        let checker = UpdateChecker::new(PackageManager::Apt);
        assert!(checker.parse_package_line("Listing...", false).is_none());
        assert!(checker.parse_package_line("Done", false).is_none());
        assert!(checker.parse_package_line("WARNING: test", false).is_none());
    }
}
```

### Testing File Operations

```rust
#[test]
fn test_nixos_mode_detection() {
    use std::fs;
    use std::io::Write;

    // Create temporary directory for test
    let temp_dir = std::env::temp_dir().join(format!("test-{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    // Test Channels mode (no flake.nix)
    let mode = PackageManagerDetector::detect_nixos_mode(temp_dir.to_str().unwrap());
    assert_eq!(mode, NixOSMode::Channels);

    // Test Flakes mode (with flake.nix)
    let flake_path = temp_dir.join("flake.nix");
    let mut file = fs::File::create(&flake_path).unwrap();
    writeln!(file, "{{}}").unwrap();

    let mode = PackageManagerDetector::detect_nixos_mode(temp_dir.to_str().unwrap());
    assert_eq!(mode, NixOSMode::Flakes);

    // Cleanup
    fs::remove_dir_all(temp_dir).unwrap();
}
```

### Running Tests

```bash
# Run all tests
cd package-updater && cargo test

# Run specific test
cargo test test_parse_arch_package_line

# Run tests with output
cargo test -- --nocapture

# Run tests with logging
RUST_LOG=debug cargo test -- --nocapture
```

---

## Code Quality Standards

### Constants Over Magic Numbers

**Always define constants for repeated values:**

```rust
// Timing constants
const STARTUP_DELAY_SECS: u64 = 2;
const POST_UPDATE_STABILIZATION_SECS: u64 = 3;
const SYNC_DEBOUNCE_SECS: u64 = 10;
const MARKER_FILE_POLL_INTERVAL_MS: u64 = 500;
const LOCK_RETRY_DELAY_SECS: u64 = 2;

// UI dimension constants
const POPUP_MIN_HEIGHT: f32 = 350.0;
const POPUP_MAX_HEIGHT: f32 = 800.0;
const POPUP_MIN_WIDTH: f32 = 450.0;
const POPUP_MAX_WIDTH: f32 = 550.0;
const PACKAGE_LIST_HEIGHT: f32 = 100.0;
```

### Regex Compilation

**Use `once_cell::Lazy` for expensive regex compilation:**

```rust
use once_cell::sync::Lazy;
use regex::Regex;

static FLAKE_UPDATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:Updated|updated)\s+input\s+['\"]?([^'\s:]+)").unwrap()
});

fn parse_flake_updates(output: &str) -> Vec<Update> {
    FLAKE_UPDATE_REGEX.captures_iter(output)
        .map(|cap| /* process match */)
        .collect()
}
```

### UI Method Decomposition

**Break large UI methods into focused helpers:**

```rust
// ❌ Bad: 150-line monolithic method
fn view_updates_tab(&self) -> Element<'_, Message> {
    let mut widgets = vec![];
    // ... 150 lines of UI code ...
    column().extend(widgets).into()
}

// ✅ Good: Decomposed into focused methods
fn view_updates_tab(&self) -> Element<'_, Message> {
    let mut widgets = vec![];
    widgets.extend(self.build_status_section());
    widgets.extend(self.build_action_buttons());
    if self.update_info.has_updates() {
        widgets.extend(self.build_package_list());
    }
    column().extend(widgets).into()
}

fn build_status_section(&self) -> Vec<Element<'_, Message>> {
    // Focused on status display
}

fn build_action_buttons(&self) -> Vec<Element<'_, Message>> {
    // Focused on button creation
}

fn build_package_list(&self) -> Vec<Element<'_, Message>> {
    // Focused on package list rendering
}
```

### Documentation Standards

**Document all public items with examples:**

```rust
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
///
/// # Examples
///
/// ```rust
/// let checker = UpdateChecker::new(PackageManager::Pacman);
/// let updates = checker.check_updates(false, &nixos_config).await?;
/// println!("Found {} updates", updates.total_updates);
/// ```
pub struct UpdateChecker {
    package_manager: PackageManager,
}
```

---

## Performance Considerations

### Async Best Practices

- Use `tokio::process::Command` for all external commands
- Avoid blocking in async contexts
- Use non-blocking locks (`FlockArg::LockExclusiveNonblock`)
- Poll with reasonable intervals (500ms for file checks)

### Memory Efficiency

- Use `&str` instead of `String` for static strings
- Return `&'static str` for icon names
- Use iterators instead of collecting when possible
- Lazy-compile regexes with `once_cell::Lazy`

### UI Rendering

- Keep package lists under 50 items for smooth scrolling
- Use fixed heights to prevent layout thrashing
- Cache formatted strings when possible
- Avoid rebuilding entire UI on every update

---

## PolicyKit Integration (v1.2.0+)

### Overview

The applet now includes PolicyKit support for secure privilege escalation, eliminating the need for passwordless sudo configuration.

### Usage Pattern

```rust
use crate::polkit;

// Execute privileged command with PolicyKit
match polkit::execute_privileged(
    "nixos-rebuild",
    &["switch", "--upgrade"],
    polkit::POLKIT_ACTION_UPDATE,
    "Authentication required to update system",
).await {
    Ok(output) => // Handle success,
    Err(e) => // Fallback to sudo or show error
}
```

### Fallback Strategy

The applet automatically:
1. Checks if PolicyKit is available (`PolkitAuth::is_available()`)
2. Attempts PolicyKit authorization if available
3. Falls back to sudo with appropriate checks if PolicyKit unavailable
4. Shows helpful error messages guiding users to proper setup

### Adding New Actions

1. **Define action in policy file** (`policy/com.github.cosmic-ext.package-updater.policy`):
   ```xml
   <action id="com.github.cosmic-ext.package-updater.my-action">
     <description>My action description</description>
     <message>Authentication required for my action</message>
     <defaults>
       <allow_active>auth_admin_keep</allow_active>
     </defaults>
   </action>
   ```

2. **Add constant in polkit.rs**:
   ```rust
   pub const POLKIT_ACTION_MY_ACTION: &str = "com.github.cosmic-ext.package-updater.my-action";
   ```

3. **Use in code**:
   ```rust
   polkit::execute_privileged(
       "command",
       &["args"],
       polkit::POLKIT_ACTION_MY_ACTION,
       "User message",
   ).await?;
   ```

### Testing

PolicyKit functionality can be tested with:
```bash
# Test availability
cd package-updater && cargo test polkit

# Manual verification
pkexec echo "PolicyKit working"
```

### Benefits

- **Security**: Fine-grained per-action permissions
- **UX**: Graphical authentication dialogs
- **Audit**: Complete trail in system logs
- **Fallback**: Graceful degradation to sudo

See `POLKIT.md` for complete documentation.

---

## Integration Testing (v1.2.0+)

### Lock Mechanism Tests

7 comprehensive integration tests verify file locking behavior:

```bash
cd package-updater
cargo test test_lock              # Run all lock tests
cargo test test_concurrent_lock   # Test specific scenario
```

### Test Coverage

- Lock acquisition and automatic release
- Concurrent access prevention
- Retry logic with async operations
- PID tracking in lock files
- Sync file notifications
- Sequential lock operations
- XDG_RUNTIME_DIR handling

### Writing Integration Tests

Use `#[tokio::test]` for async tests:

```rust
#[tokio::test]
async fn test_my_async_feature() {
    let result = my_async_function().await;
    assert!(result.is_ok());
}
```

---

## Recent Improvements

### Version 1.2.0 (2026-01-16)
- PolicyKit integration for privilege escalation
- 7 integration tests for lock mechanism
- Automatic fallback to sudo when PolicyKit unavailable
- Enhanced error messages with setup guidance
- Complete PolicyKit documentation in POLKIT.md

### Version 1.1.0 (2026-01-16)
See `CHANGES.md` for complete details on:
- Security vulnerability fixes
- Comprehensive test suite addition
- Code quality improvements
- Performance optimizations
- Documentation enhancements
