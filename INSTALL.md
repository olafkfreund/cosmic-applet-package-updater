# Installation Guide

**COSMIC Package Updater Applet**
**Version**: 1.2.0
**Last Updated**: 2026-01-16

---

## Quick Start

The fastest way to install depends on your system:

| System | Method | Command |
|--------|--------|---------|
| **NixOS** | Nix Flake | `nix build` |
| **Arch/Manjaro** | From Source | `just install` |
| **Other Linux** | From Source | `just install` |

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation Methods](#installation-methods)
   - [NixOS (Recommended)](#nixos-recommended)
   - [From Source (Universal)](#from-source-universal)
   - [Manual Installation](#manual-installation)
3. [Post-Installation](#post-installation)
4. [Configuration](#configuration)
5. [Uninstallation](#uninstallation)
6. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### System Requirements

- **OS**: Linux (any distribution)
- **Desktop**: COSMIC Desktop Environment
- **Architecture**: x86_64 (amd64)
- **RAM**: 50MB minimum
- **Disk**: 5MB for binary

### Required Dependencies

**Runtime Dependencies**:
- COSMIC Desktop Environment
- PolicyKit (polkit) - Recommended for secure authentication
- Package manager (pacman, apt, dnf, zypper, apk, flatpak, or NixOS)

**Build Dependencies** (for source installation):
- Rust toolchain (1.80+)
- cargo
- just (build tool)
- Development libraries:
  - libxkbcommon-dev
  - libwayland-dev
  - pkg-config
  - Additional COSMIC/wayland libraries

---

## Installation Methods

### NixOS (Recommended)

#### Option 1: Using Nix Flake

This is the **easiest method** for NixOS users as it handles all dependencies automatically.

```bash
# Clone repository
git clone https://github.com/cosmic-ext/cosmic-applet-package-updater.git
cd cosmic-applet-package-updater

# Build with Nix
nix build

# The binary will be in result/bin/
./result/bin/cosmic-ext-applet-package-updater

# Optional: Install to profile
nix profile install .
```

#### Option 2: Add to NixOS Configuration

Add the applet to your NixOS configuration:

```nix
# In your flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    cosmic-package-updater = {
      url = "github:cosmic-ext/cosmic-applet-package-updater";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, cosmic-package-updater, ... }: {
    nixosConfigurations.yourhostname = nixpkgs.lib.nixosSystem {
      modules = [
        {
          environment.systemPackages = [
            cosmic-package-updater.packages.${system}.default
          ];
        }
      ];
    };
  };
}
```

Then rebuild:
```bash
sudo nixos-rebuild switch
```

#### Option 3: Add to Home Manager

For user-level installation:

```nix
# In your home.nix
{
  home.packages = [
    inputs.cosmic-package-updater.packages.${pkgs.system}.default
  ];
}
```

### From Source (Universal)

This method works on any Linux distribution but requires installing build dependencies manually.

#### Step 1: Install Dependencies

**Arch Linux / Manjaro**:
```bash
sudo pacman -S rust cargo just base-devel \
    libxkbcommon wayland pkg-config
```

**Ubuntu / Debian**:
```bash
sudo apt update
sudo apt install rustc cargo just build-essential \
    libxkbcommon-dev libwayland-dev pkg-config \
    libdbus-1-dev libfontconfig-dev
```

**Fedora**:
```bash
sudo dnf install rust cargo just gcc \
    libxkbcommon-devel wayland-devel pkgconfig \
    dbus-devel fontconfig-devel
```

**openSUSE**:
```bash
sudo zypper install rust cargo just gcc \
    libxkbcommon-devel wayland-devel pkgconfig \
    dbus-1-devel fontconfig-devel
```

#### Step 2: Install Rust (if not available in repos)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable
```

#### Step 3: Install Just (if not in repos)

```bash
cargo install just
```

#### Step 4: Clone and Build

```bash
# Clone repository
git clone https://github.com/cosmic-ext/cosmic-applet-package-updater.git
cd cosmic-applet-package-updater

# Build release binary
just build-release

# Binary is now at: target/release/cosmic-ext-applet-package-updater
```

#### Step 5: Install System-Wide

```bash
# Install to /usr (requires sudo)
sudo just install

# This installs:
# - Binary: /usr/bin/cosmic-ext-applet-package-updater
# - Desktop file: /usr/share/applications/com.github.cosmic_ext.PackageUpdater.desktop
# - Metadata: /usr/share/metainfo/com.github.cosmic_ext.PackageUpdater.metainfo.xml
# - PolicyKit policy: /usr/share/polkit-1/actions/com.github.cosmic-ext.package-updater.policy
# - Icons: /usr/share/icons/hicolor/*/apps/com.github.cosmic_ext.PackageUpdater.svg
```

#### Step 6: Restart COSMIC Panel

```bash
# Restart panel to load new applet
systemctl --user restart cosmic-panel

# Or restart entire COSMIC session
# Log out and log back in
```

### Manual Installation

If you prefer to install manually without `just`:

```bash
# Build release binary
cd package-updater
cargo build --release
cd ..

# Install binary
sudo install -Dm0755 target/release/cosmic-ext-applet-package-updater \
    /usr/bin/cosmic-ext-applet-package-updater

# Install desktop file
sudo install -Dm0644 res/com.github.cosmic_ext.PackageUpdater.desktop \
    /usr/share/applications/com.github.cosmic_ext.PackageUpdater.desktop

# Install metainfo
sudo install -Dm0644 res/com.github.cosmic_ext.PackageUpdater.metainfo.xml \
    /usr/share/metainfo/com.github.cosmic_ext.PackageUpdater.metainfo.xml

# Install PolicyKit policy
sudo install -Dm0644 policy/com.github.cosmic-ext.package-updater.policy \
    /usr/share/polkit-1/actions/com.github.cosmic-ext.package-updater.policy

# Install icons
for size in 16 22 24 32 48 64 128 256; do
    sudo install -Dm0644 \
        "res/icons/hicolor/${size}x${size}/apps/com.github.cosmic_ext.PackageUpdater.svg" \
        "/usr/share/icons/hicolor/${size}x${size}/apps/com.github.cosmic_ext.PackageUpdater.svg"
done
```

---

## Post-Installation

### 1. Add to COSMIC Panel

The applet should automatically appear in your COSMIC panel after installation and panel restart. If not:

1. Right-click on COSMIC panel
2. Select "Panel Settings"
3. Click "Add Applet"
4. Find "Package Updater" and click "Add"

### 2. Verify Installation

```bash
# Check binary is installed
which cosmic-ext-applet-package-updater

# Check PolicyKit policy is installed
pkaction | grep cosmic-ext

# Test PolicyKit authentication
pkexec echo "PolicyKit working"

# Run applet manually (for testing)
cosmic-ext-applet-package-updater
```

### 3. Configure PolicyKit (Recommended)

PolicyKit is automatically configured if installed. Verify it's working:

```bash
# Check PolicyKit daemon is running
systemctl status polkit

# Check your user is in appropriate group
groups | grep -E 'wheel|sudo'

# Test authentication
pkexec true
```

If PolicyKit is not available, see [Sudo Fallback Configuration](#sudo-fallback-configuration).

### 4. Configure Check Interval

1. Click the applet icon in the panel
2. Open the "Settings" tab
3. Adjust "Check Interval" (default: 60 minutes)
4. Enable/disable automatic checking

---

## Configuration

### Application Settings

Settings are stored in: `~/.config/cosmic/com.github.cosmic_ext.PackageUpdater/`

**Available Settings**:
- **Package Manager**: Auto-detected or manually selected
- **Check Interval**: 1-1440 minutes (default: 60)
- **Auto-check on startup**: Enable/disable
- **Include AUR**: For paru/yay users only
- **Show notifications**: Enable/disable
- **Show update count**: Display count badge on icon
- **Preferred terminal**: For running updates

### PolicyKit Configuration

**Default Policy** (`/usr/share/polkit-1/actions/com.github.cosmic-ext.package-updater.policy`):
- Requires admin authentication
- Caches authorization for session
- Applies to all package manager operations

**Customize Authorization**:

Create `/etc/polkit-1/rules.d/50-package-updater.rules`:

```javascript
// Allow wheel group without password
polkit.addRule(function(action, subject) {
    if (action.id.match("com.github.cosmic-ext.package-updater.") &&
        subject.isInGroup("wheel")) {
        return polkit.Result.YES;
    }
});
```

See `POLKIT.md` for complete customization guide.

### Sudo Fallback Configuration

If PolicyKit is not available, configure passwordless sudo:

**For NixOS**:
```bash
sudo visudo -f /etc/sudoers.d/nixos-rebuild

# Add (replace %wheel with your group):
%wheel ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild

# Set permissions
sudo chmod 0440 /etc/sudoers.d/nixos-rebuild
```

**For Other Systems**:
```bash
sudo visudo -f /etc/sudoers.d/package-updater

# For APT (Debian/Ubuntu):
%sudo ALL=(ALL) NOPASSWD: /usr/bin/apt

# For DNF (Fedora):
%wheel ALL=(ALL) NOPASSWD: /usr/bin/dnf

# For Pacman (Arch):
%wheel ALL=(ALL) NOPASSWD: /usr/bin/pacman

# Set permissions
sudo chmod 0440 /etc/sudoers.d/package-updater
```

---

## Uninstallation

### Using Just

```bash
cd cosmic-applet-package-updater
sudo just uninstall
```

### Manual Uninstallation

```bash
# Remove binary
sudo rm /usr/bin/cosmic-ext-applet-package-updater

# Remove desktop file
sudo rm /usr/share/applications/com.github.cosmic_ext.PackageUpdater.desktop

# Remove metainfo
sudo rm /usr/share/metainfo/com.github.cosmic_ext.PackageUpdater.metainfo.xml

# Remove PolicyKit policy
sudo rm /usr/share/polkit-1/actions/com.github.cosmic-ext.package-updater.policy

# Remove icons
for size in 16 22 24 32 48 64 128 256; do
    sudo rm "/usr/share/icons/hicolor/${size}x${size}/apps/com.github.cosmic_ext.PackageUpdater.svg"
done

# Remove configuration (optional)
rm -rf ~/.config/cosmic/com.github.cosmic_ext.PackageUpdater/

# Restart panel
systemctl --user restart cosmic-panel
```

### NixOS Uninstallation

```bash
# If installed with nix profile
nix profile remove cosmic-ext-applet-package-updater

# If in system configuration, remove from packages list and rebuild
sudo nixos-rebuild switch
```

---

## Troubleshooting

### Applet Doesn't Appear in Panel

**Solution**:
```bash
# Restart COSMIC panel
systemctl --user restart cosmic-panel

# Or restart COSMIC compositor
systemctl --user restart cosmic-comp

# Check if applet is running
pgrep -f cosmic-ext-applet-package-updater

# Check logs
journalctl --user -u cosmic-panel -f
```

### Build Fails with Missing Dependencies

**Symptom**: `error: could not find system library 'xkbcommon'`

**Solution**:
```bash
# Install missing development libraries
# See "Install Dependencies" section for your distribution
```

### PolicyKit Authentication Fails

**Symptom**: "Authorization denied" when checking updates

**Solutions**:

1. **Verify PolicyKit is running**:
   ```bash
   systemctl status polkit
   ```

2. **Check your user group**:
   ```bash
   groups
   # Should include 'wheel' or 'sudo'
   ```

3. **Test PolicyKit manually**:
   ```bash
   pkexec echo test
   ```

4. **If PolicyKit unavailable, configure sudo** (see [Sudo Fallback](#sudo-fallback-configuration))

See `POLKIT.md` for complete troubleshooting guide.

### Permission Denied for NixOS Updates

**Symptom**: "NixOS channels mode requires passwordless sudo or PolicyKit"

**Solutions**:

1. **Recommended**: Install and configure PolicyKit (see above)

2. **Alternative**: Configure passwordless sudo:
   ```bash
   sudo visudo -f /etc/sudoers.d/nixos-rebuild
   # Add: %wheel ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild
   ```

### Updates Not Detected

**Symptom**: Applet shows "System up to date" when updates are available

**Solutions**:

1. **Verify package manager**:
   ```bash
   # Check which package manager is detected
   # Open applet → Settings tab

   # Manually test package manager
   checkupdates  # For Arch
   apt list --upgradable  # For Debian/Ubuntu
   dnf check-update  # For Fedora
   ```

2. **Check update cache**:
   ```bash
   # Update package database
   sudo pacman -Sy  # Arch
   sudo apt update  # Debian/Ubuntu
   sudo dnf check-update  # Fedora
   ```

3. **Check applet logs**:
   ```bash
   # Run with debug logging
   RUST_LOG=debug cosmic-ext-applet-package-updater
   ```

### High CPU or Memory Usage

**Symptom**: Applet using excessive resources

**Solutions**:

1. **Check configuration**:
   - Increase check interval (Settings tab)
   - Disable auto-check if not needed

2. **Profile performance**:
   ```bash
   # Monitor resource usage
   ps aux | grep cosmic-ext-applet-package-updater

   # Check for issues
   journalctl --user -u cosmic-panel | grep package-updater
   ```

See `PERFORMANCE.md` for detailed profiling guide.

---

## Getting Help

### Resources

- **Documentation**:
  - `README.md` - Project overview
  - `CLAUDE.md` - Development guide
  - `POLKIT.md` - PolicyKit integration
  - `SECURITY.md` - Security features
  - `PERFORMANCE.md` - Performance profiling
  - `CHANGES.md` - Changelog

- **Issue Tracker**: [GitHub Issues](https://github.com/cosmic-ext/cosmic-applet-package-updater/issues)

### Reporting Issues

When reporting issues, include:

1. **System Information**:
   ```bash
   # Distribution and version
   cat /etc/os-release

   # Desktop environment version
   cosmic-panel --version

   # Package manager
   pacman --version  # or apt --version, etc.
   ```

2. **Applet Version**:
   ```bash
   cosmic-ext-applet-package-updater --version
   ```

3. **Error Logs**:
   ```bash
   # Run with debug logging
   RUST_LOG=debug cosmic-ext-applet-package-updater 2>&1 | tee applet.log
   ```

4. **PolicyKit Status** (if authentication issues):
   ```bash
   systemctl status polkit
   pkaction | grep cosmic-ext
   ```

---

## Building for Distribution

### Creating a Package

**Arch Linux (PKGBUILD)**:
```bash
# See AUR package template
# https://aur.archlinux.org/packages/cosmic-ext-applet-package-updater-git
```

**Debian/Ubuntu (deb)**:
```bash
# Install packaging tools
sudo apt install dpkg-dev debhelper

# Create debian package
dpkg-buildpackage -us -uc
```

**RPM (Fedora/RHEL)**:
```bash
# Create RPM spec file
rpmbuild -ba cosmic-ext-applet-package-updater.spec
```

### Release Checklist

For maintainers creating releases:

- [ ] Update version in `Cargo.toml`
- [ ] Update `CHANGES.md` with release notes
- [ ] Run full test suite: `cargo test`
- [ ] Build release: `just build-release`
- [ ] Test installation: `sudo just install`
- [ ] Create git tag: `git tag -a v1.2.0 -m "Version 1.2.0"`
- [ ] Push tag: `git push origin v1.2.0`
- [ ] Create GitHub release with binaries

---

## Upgrade Guide

### From v1.1.0 to v1.2.0

**Changes**:
- Added PolicyKit support
- Added 7 integration tests
- Enhanced security features

**Upgrade Steps**:
```bash
# Pull latest changes
git pull origin master

# Rebuild and reinstall
just build-release
sudo just install

# Restart panel
systemctl --user restart cosmic-panel
```

**New Features**:
- PolicyKit eliminates need for sudo configuration
- Graphical authentication dialogs
- See `POLKIT.md` for details

**Breaking Changes**: None - fully backward compatible

---

**Version**: 1.2.0
**Last Updated**: 2026-01-16
**Status**: Production Ready ✅

For additional help, see documentation in the repository or open an issue on GitHub.
