# GitHub Actions Workflows

This directory contains CI/CD workflows for the COSMIC Package Updater Applet.

## Workflows

### `ci.yml` - Continuous Integration

Runs on every push and pull request to `master`/`main` branches.

**Steps:**
1. Installs Nix with flakes support
2. Configures Cachix binary cache
3. Checks flake metadata
4. Builds the NixOS package
5. Runs Clippy linting checks
6. Runs format checks
7. Verifies package builds correctly

**Cachix Integration:**
- Built artifacts are automatically pushed to Cachix
- Uses `CACHIX_KEY` secret for authentication
- Filters out source and system packages (`-source$|-sys$`)

### `release.yml` - Release Automation

Runs when a version tag (e.g., `v1.2.0`) is pushed.

**Steps:**
1. Installs Nix with flakes support
2. Configures Cachix binary cache
3. Builds the release package
4. Creates GitHub Release with auto-generated notes
5. Pushes release artifacts to Cachix

## Setup

### Cachix Configuration

1. **Create a Cachix cache** at [cachix.org](https://cachix.org):
   ```bash
   cachix create cosmic-applet-package-updater
   ```

2. **Get your authentication token**:
   - Go to your cache settings on cachix.org
   - Copy the authentication token

3. **Add GitHub secret**:
   - Go to your GitHub repository → Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Name: `CACHIX_KEY`
   - Value: Your Cachix authentication token
   - Click "Add secret"

### Testing Workflows Locally

You can test the build locally using the same Nix commands:

```bash
# Install Nix (if not already installed)
curl -L https://nixos.org/nix/install | sh

# Enable flakes
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# Run the same checks as CI
nix flake check --print-build-logs
nix build .#default --print-build-logs
nix build .#checks.x86_64-linux.clippy --print-build-logs
nix build .#checks.x86_64-linux.fmt --print-build-logs
```

## Cachix Usage

### Installing from Cachix

Users can install pre-built binaries from Cachix to speed up installation:

```bash
# Add the binary cache
cachix use cosmic-applet-package-updater

# Install using Nix
nix profile install github:olafkfreund/cosmic-applet-package-updater
```

Or with NixOS configuration:

```nix
{
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
}
```

### Benefits of Cachix

- **Fast installations**: No need to compile from source
- **Consistent builds**: Same binaries used in CI/CD
- **Reduced build times**: CI/CD pipeline is faster with caching
- **Bandwidth savings**: Binary caches are distributed via CDN

## Troubleshooting

### Workflow fails with "CACHIX_KEY not found"

Make sure you've added the `CACHIX_KEY` secret to your repository settings.

### Cachix authentication fails

Verify that:
1. Your Cachix token is valid and not expired
2. The cache name matches: `cosmic-applet-package-updater`
3. The token has write permissions for the cache

### Build fails on specific check

Run the failing check locally:
```bash
# For clippy
nix build .#checks.x86_64-linux.clippy --print-build-logs

# For format
nix build .#checks.x86_64-linux.fmt --print-build-logs
```

### Cachix push is slow

The `pushFilter` setting excludes source and system packages to reduce upload size. If needed, adjust this in the workflow file.

## References

- [Cachix Documentation](https://docs.cachix.org/)
- [cachix/install-nix-action](https://github.com/cachix/install-nix-action)
- [cachix/cachix-action](https://github.com/cachix/cachix-action)
- [Nix Flakes](https://nixos.wiki/wiki/Flakes)
- [GitHub Actions](https://docs.github.com/en/actions)
