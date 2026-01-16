# GitHub Actions Workflows

Improved CI/CD workflows based on working patterns from cosmic-applet-music-player.

## Workflows

### `ci.yml` - Continuous Integration

**Triggers:**
- Push to `master`/`main` branches
- Pull requests to `master`/`main`
- Manual workflow dispatch
- Ignores documentation and metadata files

**Jobs:**

1. **nix-build** - Main build and checks
   - Builds the package for x86_64-linux
   - Runs clippy checks separately
   - Runs format checks separately
   - Uses `--show-trace` for better debugging

2. **nix-flake-check** - Flake validation
   - Validates flake.nix syntax and structure
   - Runs all checks defined in the flake

3. **build-dev-shell** - Development environment
   - Builds the development shell
   - Verifies development tools are available

4. **summary** - CI summary
   - Aggregates results from all jobs
   - Provides clear pass/fail status

**Key Features:**
- ✅ Concurrency control (cancels duplicate runs)
- ✅ Only pushes to Cachix on master/main branch
- ✅ Separate jobs for better parallelization
- ✅ Uses install-nix-action@v31 (latest stable)
- ✅ Better error reporting with --show-trace

### `release.yml` - Release Automation

**Triggers:**
- Push of version tags (v*.*.*)
- Manual workflow dispatch with tag input

**Jobs:**

1. **build-release** - Release build
   - Builds release package for x86_64-linux
   - Runs all checks
   - Generates build metadata
   - Uploads artifacts for 90 days
   - Always pushes to Cachix

2. **create-github-release** - GitHub release
   - Creates GitHub release with notes
   - Includes installation instructions
   - Attaches build metadata

3. **release-summary** - Release summary
   - Aggregates release status
   - Provides links to release and Cachix

**Key Features:**
- ✅ Automatic release notes generation
- ✅ Build metadata with commit info
- ✅ Artifact retention for 90 days
- ✅ Always pushes release builds to Cachix

## Configuration

### Required Secrets

- **`CACHIX_KEY`**: Authentication token for Cachix
  - Get from: https://app.cachix.org
  - Add to: Repository Settings → Secrets → Actions
  - Name: `CACHIX_KEY`
  - Value: Your Cachix authentication token

### Cachix Cache

Cache name: `cosmic-applet-package-updater`
Public key: `cosmic-applet-package-updater.cachix.org-1:34TyvdAddZx+Ngn9LhYRcsUB3yjgTuT+8VAuFW0WmcM=`

## Usage

### For Users

Install from Cachix (fast, no compilation):

```bash
# Add the binary cache
cachix use cosmic-applet-package-updater

# Install the package
nix profile install github:olafkfreund/cosmic-applet-package-updater
```

### For NixOS Users

Add to your configuration:

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

  environment.systemPackages = [
    (pkgs.callPackage (builtins.fetchGit {
      url = "https://github.com/olafkfreund/cosmic-applet-package-updater";
      ref = "master";
    }) {})
  ];
}
```

## Improvements Over Previous Workflows

### Better Structure
- ✅ Separate jobs for build, checks, and validation
- ✅ Parallel execution where possible
- ✅ Clear job names and descriptions
- ✅ Summary job for overall status

### Better Performance
- ✅ Concurrency control cancels duplicate runs
- ✅ Only pushes to Cachix when needed (master/main or releases)
- ✅ Path ignores for documentation changes
- ✅ Separate check jobs can be cached independently

### Better Debugging
- ✅ `--show-trace` flag for detailed error traces
- ✅ `--log-format bar-with-logs` for better output
- ✅ Separate steps show exactly where failures occur
- ✅ Build metadata includes commit info

### Better Reliability
- ✅ Uses latest stable action versions (v31, v15, v4)
- ✅ Proper error handling and status checks
- ✅ Validation of dev shell ensures reproducibility
- ✅ Summary job catches any failures

## Local Testing

Test the same builds locally:

```bash
# Build the package
nix build .#cosmic-ext-applet-package-updater --print-build-logs --show-trace

# Run clippy checks
nix build .#checks.x86_64-linux.clippy --print-build-logs --show-trace

# Run format checks
nix build .#checks.x86_64-linux.fmt --print-build-logs --show-trace

# Run flake check
nix flake check --print-build-logs --show-trace

# Test dev shell
nix develop --command bash -c "rustc --version && cargo --version"
```

## Troubleshooting

### Workflow fails on clippy/fmt checks

The workflow runs checks separately for better error reporting. If checks fail:

```bash
# Run locally to see the exact error
nix build .#checks.x86_64-linux.clippy --print-build-logs --show-trace
nix build .#checks.x86_64-linux.fmt --print-build-logs --show-trace
```

### Cachix push fails

Make sure:
1. `CACHIX_KEY` secret is set correctly
2. Cache name matches: `cosmic-applet-package-updater`
3. Token has write permissions

### Build succeeds locally but fails in CI

Check:
1. All files are committed (CI uses git checkout)
2. Flake lock is up to date
3. No local-only dependencies or paths

## References

- [Nix Flakes](https://nixos.wiki/wiki/Flakes)
- [Cachix Documentation](https://docs.cachix.org/)
- [GitHub Actions](https://docs.github.com/en/actions)
- [install-nix-action](https://github.com/cachix/install-nix-action)
- [cachix-action](https://github.com/cachix/cachix-action)
