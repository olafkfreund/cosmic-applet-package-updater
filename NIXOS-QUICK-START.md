# NixOS Quick Start Guide

## 🚀 Pure Nix Build (Recommended)

```bash
# Clone the repository
git clone https://github.com/olafkfreund/cosmic-applet-package-updater
cd cosmic-applet-package-updater

# Build with Nix (uses crane for automatic git dependency handling)
nix build

# Install to your profile
nix profile install

# Or run directly
nix run
```

## 🏗️ Add to Your NixOS System

Add to your `flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    cosmic-package-updater.url = "github:olafkfreund/cosmic-applet-package-updater";
  };

  outputs = { nixpkgs, cosmic-package-updater, ... }: {
    nixosConfigurations.your-hostname = nixpkgs.lib.nixosSystem {
      modules = [
        ({ pkgs, ... }: {
          environment.systemPackages = [
            cosmic-package-updater.packages.${pkgs.system}.default
          ];
        })
      ];
    };
  };
}
```

Then rebuild:

```bash
sudo nixos-rebuild switch --flake .#your-hostname
```

## 🔄 Restart COSMIC

```bash
# Logout and login, or:
cosmic-panel --replace &
```

## 📍 Add to Panel

Right-click panel → Add Applet → Package Updater

## ⚙️ Configure NixOS Settings

After installation, click the applet → Settings tab:

1. **NixOS Mode**: Auto-detected (Flakes or Channels)
2. **Config Path**: Default `/etc/nixos`
3. **Check Interval**: How often to check for updates

## 🔑 Enable Passwordless Update Checks (Channels Mode)

Create `/etc/sudoers.d/nixos-rebuild`:

```bash
# Allow dry-run checks without password
%wheel ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild dry-activate*
```

## 🔄 Updating the Applet

```bash
cd cosmic-applet-package-updater
git pull
nix build
nix profile upgrade cosmic-ext-applet-package-updater
```

## 📖 Full Documentation

- **Build Solution Details**: [NIX-BUILD-SOLUTION.md](NIX-BUILD-SOLUTION.md)
- **Complete Guide**: [README-NIXOS.md](README-NIXOS.md)
- **General README**: [README.md](README.md)

## 🛠️ Development

```bash
# Enter development shell
nix develop

# Build release with cargo
just build-release

# Or build with Nix
nix build

# Run with logging
RUST_LOG=debug nix run

# Run checks
nix flake check
```

## ✅ CI Checks

```bash
nix build .#checks.x86_64-linux.clippy  # Clippy check
nix build .#checks.x86_64-linux.fmt     # Format check
nix flake check                         # All checks
```

## ❓ Troubleshooting

### Applet Not Showing?

```bash
# Check installation
which cosmic-ext-applet-package-updater

# Restart panel
cosmic-panel --replace &

# Update desktop database (if installed to ~/.local)
update-desktop-database ~/.local/share/applications
```

### NixOS Updates Not Detected?

```bash
# Check with logging
RUST_LOG=debug cosmic-ext-applet-package-updater

# Verify config path in Settings
# Ensure mode matches your setup (Flakes/Channels)
```

## 📊 Features

- ✅ Auto-detects Flakes vs Channels
- ✅ Dry-run updates (no system changes)
- ✅ Shows update count in panel
- ✅ One-click terminal updates
- ✅ Configurable check intervals
- ✅ Multi-instance sync

## 🔧 Technical Achievement

**We solved the complex git workspace dependency problem!**

This project now builds successfully with **crane** instead of `buildRustPackage`:
- ✅ Automatic git dependency handling (no manual hashes)
- ✅ Full workspace support
- ✅ Incremental build caching
- ✅ CI checks included (clippy, fmt)

See [NIX-BUILD-SOLUTION.md](NIX-BUILD-SOLUTION.md) for the complete technical solution.
