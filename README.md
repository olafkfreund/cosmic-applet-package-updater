# Package Updater Applet for COSMIC™

**Version**: 1.3.0 | **Status**: Production Ready

A lightweight and efficient package update notifier applet for the COSMIC™ desktop. Stay informed about system updates with real-time notifications and seamless integration into your COSMIC panel.

Supports multiple Linux distributions including Arch Linux, Debian/Ubuntu, Fedora, openSUSE, Alpine, and more!

![Main Interface](screenshots/Package-Updater-Main.png)

## Features

### Package Manager Support
- **Arch Linux**: Pacman, Paru, Yay (with AUR support)
- **Debian/Ubuntu/Pop!_OS**: APT
- **Fedora/RHEL**: DNF
- **openSUSE/SUSE**: Zypper
- **Alpine Linux**: APK
- **NixOS**: Channels and Flakes support ([see NixOS guide](README-NIXOS.md))
- **Universal**: Flatpak
- **Auto-detection**: Automatically discovers available package managers on first launch

### Update Management
- **Visual Indicators**: Panel icon changes based on update status
  - Package icon: System up to date
  - Update icon with count: Updates available
  - Refresh icon: Checking for updates
  - Error icon: Error occurred
- **Automatic Checking**: Configurable interval-based update checking (default: 60 minutes)
- **One-Click Updates**: Launch system updates directly from the applet in your preferred terminal
- **Detailed Package List**: View all available updates with version information (AUR packages shown separately on Arch-based systems)
- **Instance Synchronization**: Multiple applet instances stay in sync automatically

### User Interface
- **Clean Two-Tab Layout** with proper COSMIC button styling:
  - **Updates Tab**: Shows update status, package list, and action buttons
  - **Settings Tab**: Configure all preferences in one place
- **COSMIC Radio Widgets**: Native radio buttons for package manager and NixOS mode selection
- **Visual Package Illustration**: Dynamic icon showing current status
- **Explicit State Machine**: `CheckState` enum tracks Idle/Checking/Completed/Error states cleanly
- **Smart Button Placement**: Update System button appears only when updates are available
- **Scrollable Package List**: View all updates in an organized, themed list container

### Configuration Options
- **Package Manager Selection**: Choose from detected package managers
- **Check Interval**: Set how often to check for updates (1-1440 minutes)
- **Auto-check on Startup**: Automatically check for updates when applet starts
- **Include AUR Updates**: Toggle AUR package update detection (Arch Linux only)
- **Show Notifications**: Enable/disable update notifications (feature ready)
- **Show Update Count**: Display the number of updates in the panel icon
- **Preferred Terminal**: Set your preferred terminal emulator (default: cosmic-term)
- **NixOS Hostname**: Configure hostname for multi-host flake setups (with auto-detect)

### Quick Actions
- **Left Click**: Open the applet popup window
- **Middle Click on Panel Icon**: Launch system update directly
- **Update System Button**: Opens terminal with update command, then automatically re-checks

### Smart Background Operations
- **File-Based Locking**: Prevents multiple instances from checking simultaneously
- **Automatic Retry Logic**: Retries failed checks once to handle temporary errors
- **File Watcher Sync**: When one instance checks for updates, all others sync within 100ms
- **Post-Update Check**: Automatically re-checks for updates after terminal closes (3-second stabilization delay)
- **Resource Efficient**: Minimal system impact when idle

## Screenshots

### Updates Tab
![Updates Available](screenshots/Package-Updater-Updates.png)
*The main updates tab showing available updates with package details*

### Settings Tab
![Settings](screenshots/Package-Updater-Settings.png)
*Configure package manager, intervals, and preferences*

## Documentation

**Version**: 1.3.0

Comprehensive documentation is available for all aspects of the applet:

### For Users
- **[INSTALL.md](INSTALL.md)** - Complete installation guide for all Linux distributions
  - NixOS installation (flakes, configuration, Home Manager)
  - Source installation for any distribution
  - Manual installation steps
  - Post-installation configuration
  - Troubleshooting common issues

- **[POLKIT.md](POLKIT.md)** - PolicyKit integration guide
  - Security benefits and architecture
  - Installation and verification
  - Customization options
  - Troubleshooting authentication issues
  - Comparison with sudo

### For Developers
- **[CLAUDE.md](CLAUDE.md)** - Development guide and architecture
  - Code organization and patterns
  - PolicyKit integration patterns
  - Testing approaches
  - Performance considerations
  - Common pitfalls and anti-patterns

- **[PERFORMANCE.md](PERFORMANCE.md)** - Performance profiling guide
  - Profiling tools setup (perf, valgrind, hyperfine)
  - Benchmark scenarios
  - Optimization checklist
  - Performance monitoring
  - Troubleshooting performance issues

### For Security Teams
- **[SECURITY.md](SECURITY.md)** - Security policy and features
  - Implemented security protections
  - Threat model
  - Security audit results
  - Vulnerability disclosure process
  - Security best practices

### Release Information
- **[CHANGES.md](CHANGES.md)** - Complete changelog
  - v1.3.0: CheckState enum, hostname UI, radio widgets, dead code removal
  - v1.2.0: PolicyKit integration, integration tests
  - v1.1.0: Security fixes, comprehensive testing

- **[REVIEW_SUMMARY.md](REVIEW_SUMMARY.md)** - Security audit summary
- **[VERSION_1.2.0_SUMMARY.md](VERSION_1.2.0_SUMMARY.md)** - v1.2.0 release notes
- **[SESSION_SUMMARY.md](SESSION_SUMMARY.md)** - Complete development summary
- **[FINAL_SUMMARY.md](FINAL_SUMMARY.md)** - Complete session summary

### For Planning & Future Development
- **[ROADMAP.md](ROADMAP.md)** - Development roadmap
  - v1.4.0: Extended package manager support
  - v1.5.0: Real-time updates
  - v2.0.0: Intelligence & automation

### Key Features (v1.3.0)

**Security**:
- Zero critical vulnerabilities
- PolicyKit integration for secure privilege escalation
- Fine-grained per-action permissions
- Command injection prevention with `shell-escape`
- Atomic `flock`-based file locking
- Executable path validation (system dirs only)

**Testing**:
- 25+ comprehensive tests (18 unit + 7 integration)
- Async concurrency testing with tokio
- Lock mechanism verification

**Code Quality** (v1.3.0):
- `CheckState` enum replaces implicit boolean/option state fields
- Native COSMIC `radio` widgets for all selection UI
- `update_config()` DRY helper eliminates repetitive config mutation
- `LazyLock` for compiled regex (Rust 1.80+, no `once_cell`)
- Shared `paths` module for XDG runtime dir helpers
- Removed 387 lines of dead code (unused `virtualized_list` module + 5 unused dependencies)

## Installation

### From AUR (Recommended)

Install using your preferred AUR helper:

```bash
# Using paru
paru -S cosmic-applet-package-updater-git

# Using yay
yay -S cosmic-applet-package-updater-git
```

**AUR Package**: [cosmic-applet-package-updater-git](https://aur.archlinux.org/packages/cosmic-applet-package-updater-git)

### NixOS Installation (Recommended for NixOS users)

#### Quick Install with Cachix

For the fastest installation using pre-built binaries from Cachix:

```bash
# Add the Cachix binary cache
cachix use cosmic-applet-package-updater

# Install directly from GitHub
nix profile install github:olafkfreund/cosmic-applet-package-updater
```

#### NixOS Configuration

Add to your NixOS configuration to enable the binary cache and install the applet:

```nix
{ config, pkgs, ... }:

{
  # Enable Cachix binary cache for faster installations
  nix.settings = {
    substituters = [
      "https://cache.nixos.org"
      "https://cosmic-applet-package-updater.cachix.org"
    ];
    trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      "cosmic-applet-package-updater.cachix.org-1:34TyvdAddZx+Ngn9LhYRcsUB3yjgTuT+8VAuFW0WmcM="
    ];
  };

  # Install the applet
  environment.systemPackages = [
    (pkgs.callPackage (builtins.fetchGit {
      url = "https://github.com/olafkfreund/cosmic-applet-package-updater";
      ref = "master";
    }) {})
  ];
}
```

**Alternative: Using Flakes in NixOS configuration**

```nix
{
  description = "NixOS configuration with COSMIC Package Updater";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    cosmic-applet-package-updater.url = "github:olafkfreund/cosmic-applet-package-updater";
  };

  outputs = { self, nixpkgs, cosmic-applet-package-updater, ... }: {
    nixosConfigurations.yourhostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        {
          # Enable Cachix cache
          nix.settings = {
            substituters = [
              "https://cache.nixos.org"
              "https://cosmic-applet-package-updater.cachix.org"
            ];
            trusted-public-keys = [
              "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
              "cosmic-applet-package-updater.cachix.org-1:your-public-key-here"
            ];
          };

          # Install the applet
          environment.systemPackages = [
            cosmic-applet-package-updater.packages.x86_64-linux.default
          ];
        }
      ];
    };
  };
}
```

**Benefits of using Cachix:**
- **Fast installations**: Pre-built binaries, no compilation needed
- **Automatic updates**: Same binaries used in CI/CD
- **Reduced disk usage**: Shared dependencies across packages
- **CDN distribution**: Fast downloads from anywhere

#### Home Manager Configuration

If you use Home Manager, add to your `home.nix`:

```nix
{ config, pkgs, ... }:

{
  home.packages = [
    (pkgs.callPackage (builtins.fetchGit {
      url = "https://github.com/olafkfreund/cosmic-applet-package-updater";
      ref = "master";
    }) {})
  ];
}
```

**Note**: The Cachix public key is already included in the configurations above. If you need to verify it, run `cachix use cosmic-applet-package-updater` or visit https://app.cachix.org/cache/cosmic-applet-package-updater

### Build from Source

1. **Clone the repository**:
   ```bash
   git clone https://github.com/olafkfreund/cosmic-applet-package-updater.git
   cd cosmic-applet-package-updater
   ```

2. **Build and install** (using just):
   ```bash
   just build-release
   sudo just install
   ```

   Or manually with cargo:
   ```bash
   cd package-updater
   cargo build --release
   sudo install -Dm755 target/release/cosmic-ext-applet-package-updater /usr/bin/cosmic-ext-applet-package-updater
   ```

### Prerequisites

#### All Distributions
- **Desktop Environment**: COSMIC™ desktop
- **Rust**: 1.80 or newer (for building from source)
- **Terminal Emulator**: cosmic-term (recommended) or any terminal supporting `-e` flag

#### Build Dependencies

**Arch Linux / Manjaro:**
```bash
sudo pacman -S rust cargo base-devel
```

**Debian / Ubuntu / Pop!_OS:**
```bash
sudo apt install cargo libxkbcommon-dev pkg-config
```

**Fedora / RHEL:**
```bash
sudo dnf install cargo libxkbcommon-devel pkgconfig
```

**openSUSE:**
```bash
sudo zypper install cargo libxkbcommon-devel pkg-config
```

#### Runtime Dependencies (Distribution-Specific)

**Arch Linux:**
- `pacman-contrib` (for `checkupdates` command)
- Optional: `paru` or `yay` for AUR support

**Debian/Ubuntu/Pop!_OS:**
- `apt` (pre-installed)

**Fedora/RHEL:**
- `dnf` (pre-installed)

**openSUSE:**
- `zypper` (pre-installed)

**Alpine:**
- `apk` (pre-installed)

**Universal (any distribution):**
- `flatpak` (optional)

## Usage

### Adding the Applet to COSMIC Panel

After installation, add the Package Updater applet to your COSMIC panel:

1. Right-click on the COSMIC panel
2. Select "Panel Settings" or "Configure Panel"
3. Find "Package Updater" in the available applets list
4. Click to add it to your panel

The applet will appear as an icon in your COSMIC panel.

### Using the Applet

**Updates Tab**:
- View current update status with visual indicators
- See detailed package list with version information
- Packages are organized into Official and AUR categories
- Click "Check for Updates" to manually refresh
- Click "Update System" to launch updates in terminal (appears only when updates available)
- Tip displayed: "Middle-click on the Panel icon" for quick updates

**Settings Tab**:
- **Package Manager**: Select from detected package managers
- **Check Interval**: Set minutes between automatic checks (1-1440)
- **Auto-check on startup**: Toggle automatic checking when applet starts
- **Include AUR updates**: Enable/disable AUR package detection (only shown on Arch Linux with Paru/Yay)
- **Show notifications**: Enable/disable update notifications
- **Show update count**: Toggle update count badge on panel icon
- **Preferred Terminal**: Set terminal command (default: cosmic-term)

**Quick Actions**:
- **Left Click**: Open/close applet popup
- **Middle Click on Panel Icon**: Launch system update immediately

### How Updates Work

1. **Checking for Updates**:
   - Automatic checks run based on your configured interval
   - Manual checks via "Check for Updates" button
   - File-based locking prevents simultaneous checks across instances

2. **Installing Updates**:
   - Click "Update System" or middle-click the panel icon
   - Terminal opens with update command for your package manager
   - Complete the update process in the terminal
   - Close terminal when done
   - Applet automatically re-checks for updates after 3 seconds
   - All applet instances sync the new state within 100ms

3. **Instance Synchronization**:
   - Multiple applet instances stay synchronized automatically
   - When one instance checks for updates, others sync via file watcher
   - Prevents duplicate checks with file-based locking

## Configuration

Settings are stored in:
```
~/.config/cosmic/com.github.cosmic_ext.PackageUpdater/
```

Lock and sync files (automatically managed):
```
$XDG_RUNTIME_DIR/cosmic-package-updater.lock
$XDG_RUNTIME_DIR/cosmic-package-updater.sync
```

## How It Works

### Update Detection

The applet uses distribution-specific commands to detect updates:

**Arch Linux:**
- **Official Packages**: `checkupdates` (from pacman-contrib)
- **AUR Packages (Paru)**: `paru -Qu --aur`
- **AUR Packages (Yay)**: `yay -Qu --aur`

**Debian/Ubuntu/Pop!_OS:**
- `apt list --upgradable`

**Fedora/RHEL:**
- `dnf check-update -q`

**openSUSE/SUSE:**
- `zypper list-updates`

**Alpine:**
- `apk -u list`

**Flatpak:**
- `flatpak remote-ls --updates`

**NixOS:**
- **Channels Mode**: `sudo nixos-rebuild dry-activate --upgrade`
- **Flakes Mode**: `nix flake update --dry-run`

### NixOS Support

The applet now fully supports NixOS with both traditional channels and modern flakes!

**Supported Modes:**
- **Channels**: Traditional NixOS update mechanism using `nix-channel`
- **Flakes**: Modern reproducible configuration approach using `flake.nix` and `flake.lock`

**Configuration:**
1. Select "nixos" from Package Managers in Settings
2. Choose your mode: Flakes or Channels (radio buttons)
3. Set your NixOS configuration path (default: `/etc/nixos`)
4. Set hostname for multi-host flake setups (or click "Detect" to auto-detect from `/etc/hostname`)
5. Click "Auto-detect Mode" to automatically detect your setup based on presence of `flake.nix`

**Requirements:**
- NixOS system with `nixos-rebuild` available
- For update checks with channels: passwordless sudo configured (see below)
- For flakes: Valid `flake.nix` and `flake.lock` in config directory
- For flakes mode: `nix` command with flakes support enabled

**Passwordless Sudo Setup (Channels Mode):**

For channels mode to check for updates without password prompts, configure passwordless sudo:

Create `/etc/sudoers.d/nixos-rebuild` with:
```
%wheel ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild
```

Or if you're not in the wheel group:
```
your_username ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild
```

**How Updates Work:**
- **Channels**: Runs `nixos-rebuild dry-activate --upgrade` to check what systemd units/services would change
- **Flakes**: Runs `nix flake update --dry-run` to check which flake inputs have newer versions available

**Update Command:**
- **Channels**: `sudo nix-channel --update && sudo nixos-rebuild switch --upgrade`
- **Flakes**: `cd <config_path> && nix flake update && sudo nixos-rebuild switch --flake .#<hostname>`

**Note on Update Display:**
- NixOS is declarative, so instead of showing individual package updates like other distributions, the applet shows:
  - **Channels**: System services/units that would change (start, restart, reload, stop)
  - **Flakes**: Flake inputs that have new versions (e.g., `flake:nixpkgs abc1234 → def5678`)

### Smart Features

- **Retry Logic**: Failed checks are automatically retried once after 1 second
- **Exit Code Handling**: Correctly interprets exit codes (2 for checkupdates, 1 for paru/yay means no updates)
- **File-Based Locking**: Uses `$XDG_RUNTIME_DIR/cosmic-package-updater.lock` to prevent simultaneous checks
- **File Watcher Sync**: Monitors `$XDG_RUNTIME_DIR/cosmic-package-updater.sync` to sync instances
- **Debouncing**: 10-second minimum between syncs to prevent rapid repeated checks

## Technical Details

- **Framework**: Built with libcosmic
- **Language**: Rust
- **Async Operations**: All package manager calls are non-blocking (tokio)
- **Configuration**: Persistent settings with cosmic-config
- **File Watching**: Uses the `notify` crate for instance synchronization

## Troubleshooting

### Applet not appearing in panel
- Ensure the applet is properly installed: `which cosmic-ext-applet-package-updater` should return a path
- Restart the COSMIC panel or log out and back in
- Check COSMIC Settings → Desktop → Panel settings

### No package managers found
- **Arch Linux**: Install `pacman-contrib` for the `checkupdates` command: `sudo pacman -S pacman-contrib`
- **Arch Linux (AUR)**: Install `paru` or `yay` for AUR support
- **Other distros**: The default package manager (apt/dnf/zypper/apk) should be pre-installed
- Click "Discover Package Managers" button in the Settings tab
- Ensure package managers are in your `$PATH`

### Updates not showing correctly
- **Arch Linux**: Verify `checkupdates` works from command line: `checkupdates`
- **Debian/Ubuntu**: Try `apt list --upgradable` from command line
- **Fedora**: Try `dnf check-update` from command line
- Check that the correct package manager is selected in Settings
- Try clicking "Check for Updates" manually
- Check system logs for error messages

### Applet keeps checking repeatedly on startup
- This was a bug that has been fixed
- The first sync event on startup is now ignored
- Only syncs when last check was more than 3 seconds ago

### Multiple instances out of sync
- The file watcher should automatically sync all instances
- If issues persist, remove sync file: `rm $XDG_RUNTIME_DIR/cosmic-package-updater.sync`
- Restart the applet

### Terminal not launching
- Verify the preferred terminal setting in Settings tab
- Ensure the terminal is installed: `which cosmic-term`
- Try a different terminal like `konsole` or `kitty`

### "Update check already in progress" errors
- Another instance is currently checking for updates
- The lock file prevents simultaneous checks
- Wait a few seconds and try again
- If persistent, remove lock file: `rm $XDG_RUNTIME_DIR/cosmic-package-updater.lock`

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.

## License

This project is licensed under the GPL-3.0 License - see the LICENSE file for details.

## Credits

Developed for the COSMIC™ desktop community.
