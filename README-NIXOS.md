# NixOS Integration Guide

This document explains how to install and use the COSMIC Package Updater Applet on NixOS.

## Installation Methods

### Method 1: Quick Install from Pre-built Binary (Recommended)

Since the project uses complex git dependencies that are challenging for Nix to vendor, the easiest approach is to build outside the Nix sandbox and install locally:

```bash
# Enter development environment with all dependencies
nix develop

# Build the release binary
just build-release

# Install to your local profile
mkdir -p ~/.local/bin ~/.local/share/applications ~/.local/share/metainfo ~/.local/share/icons/hicolor
cp target/release/cosmic-ext-applet-package-updater ~/.local/bin/
cp res/com.github.cosmic_ext.PackageUpdater.desktop ~/.local/share/applications/
cp res/com.github.cosmic_ext.PackageUpdater.metainfo.xml ~/.local/share/metainfo/
for size in $(ls res/icons/hicolor); do
  mkdir -p ~/.local/share/icons/hicolor/$size/apps
  cp res/icons/hicolor/$size/apps/com.github.cosmic_ext.PackageUpdater.svg \
     ~/.local/share/icons/hicolor/$size/apps/
done

# Update desktop database
update-desktop-database ~/.local/share/applications
```

### Method 2: System-Wide Installation (NixOS Configuration)

Add to your NixOS configuration to make the development tools available system-wide, then build and install:

```nix
# In configuration.nix
{ config, pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    # Rust toolchain for building
    cargo
    rustc
    pkg-config
    just

    # Required libraries
    libxkbcommon
    wayland
  ];
}
```

Then build and install as shown in Method 1.

### Method 3: Development Shell

For development work:

```bash
# Clone the repository
git clone https://github.com/olafkfreund/cosmic-applet-package-updater
cd cosmic-applet-package-updater

# Enter development shell
nix develop

# Available commands:
just build-release  # Build release version
just run           # Run with debug logging
just check         # Run clippy checks
```

## Features for NixOS Users

The applet includes special support for NixOS:

### Automatic Detection
- **Flakes Mode**: Automatically detected if `flake.nix` exists in your config path
- **Channels Mode**: Used if no flake.nix is found

### Update Checking
- **Flakes**: Uses `nix flake update --dry-run` to check for input updates
- **Channels**: Uses `nixos-rebuild dry-activate --upgrade` to preview updates
- **No System Changes**: All checks are dry-runs - they show what would update without actually updating

### Configuration

Click the applet → Settings tab to configure:

1. **NixOS Mode**: Choose Flakes or Channels (auto-detects by default)
2. **Config Path**: Set your NixOS configuration path (default: `/etc/nixos`)
3. **Auto-detect**: Click to automatically detect your mode

### Permissions for NixOS Update Checking

For **Channels mode**, the applet needs to run `sudo nixos-rebuild dry-activate --upgrade`.

To enable passwordless checks, add this to `/etc/sudoers.d/nixos-rebuild`:

```bash
# Allow wheel group to run nixos-rebuild dry-activate without password
%wheel ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild dry-activate*
```

For **Flakes mode**, no special permissions are needed - it just reads your flake.lock file.

## After Installation

1. **Restart COSMIC Panel**:
   ```bash
   cosmic-panel --replace &
   # Or logout and login
   ```

2. **Add to Panel**:
   - Right-click on the COSMIC panel
   - Select "Add Applet"
   - Look for "Package Updater"

3. **Configure NixOS Settings**:
   - Click the applet
   - Go to Settings tab
   - Configure mode and path

## Updating the Applet

```bash
cd cosmic-applet-package-updater
git pull
nix develop
just build-release

# Reinstall (copy new binary)
cp target/release/cosmic-ext-applet-package-updater ~/.local/bin/
```

## Development

### Build and Test

```bash
# Enter development environment
nix develop

# Build release version
just build-release

# Run with debug logging
RUST_LOG=debug just run

# Run clippy checks
just check
```

### Development Tools Available in `nix develop`

- Rust toolchain (cargo, rustc, rustfmt, clippy)
- System libraries (libxkbcommon, wayland, libGL)
- Build tools (pkg-config, just)

## Troubleshooting

### Applet Not Showing in Panel

1. **Restart COSMIC Panel**:
   ```bash
   cosmic-panel --replace &
   ```

2. **Check Installation**:
   ```bash
   which cosmic-ext-applet-package-updater
   ls ~/.local/share/applications/com.github.cosmic_ext.PackageUpdater.desktop
   ```

3. **Update Desktop Database**:
   ```bash
   update-desktop-database ~/.local/share/applications
   ```

### NixOS Updates Not Showing

1. **Check Config Path**: Verify the path in Settings matches your config location
2. **Check Mode**: Ensure Flakes/Channels mode matches your setup
3. **Check Permissions**: For Channels mode, ensure sudo is configured (see above)
4. **Check Logs**: Run from terminal to see error messages:
   ```bash
   RUST_LOG=debug cosmic-ext-applet-package-updater
   ```

### Build Errors

If you encounter build errors:

1. **Clean Build**:
   ```bash
   cargo clean
   just build-release
   ```

2. **Update Dependencies**:
   ```bash
   cargo update
   just build-release
   ```

## How It Works

### For Flakes Users

1. Reads your `flake.lock` file
2. Runs `nix flake update --dry-run` in your config directory
3. Parses the output to detect which inputs would be updated
4. Shows count of pending updates in the panel

### For Channels Users

1. Runs `sudo nixos-rebuild dry-activate --upgrade`
2. Parses the rebuild output to find packages that would be updated
3. Shows count of pending updates in the panel

## Technical Notes

### Why Not Pure Nix Build?

This project depends on libcosmic and other COSMIC libraries that use complex git workspace structures. Nix's `importCargoLock` currently has limitations with these complex workspace dependencies.

The recommended approach is to use `nix develop` to get a reproducible build environment with all dependencies, then build with `cargo`/`just` which handles the git dependencies correctly.

### Future Improvements

As Nix's Rust build support improves, we may be able to create a pure Nix derivation. For now, the development shell provides a reproducible environment while allowing cargo to handle the complex dependency resolution.

## Contributing

To contribute NixOS-specific improvements:

1. Fork the repository
2. Make your changes
3. Test with `nix develop` and `just build-release`
4. Submit a pull request

The NixOS implementation is in:
- `package-updater/src/config.rs` - Configuration structures
- `package-updater/src/package_manager.rs` - Update detection logic
- `package-updater/src/app.rs` - UI components
