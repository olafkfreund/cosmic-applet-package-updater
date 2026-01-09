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
