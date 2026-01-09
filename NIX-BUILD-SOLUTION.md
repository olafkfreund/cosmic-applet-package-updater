# Nix Build Solution - How We Solved the Git Dependency Problem

## Problem Statement

The COSMIC Package Updater uses complex git workspace dependencies (libcosmic, iced, etc.) that were failing to build with Nix's standard `buildRustPackage` due to `importCargoLock` limitations with nested workspace structures.

**Error encountered**:
```
Cannot find path for crate 'iced-0.14.0-dev' in the tree in: /nix/store/...-libcosmic-...
```

## Solution: Crane

We switched from `buildRustPackage` to **[crane](https://github.com/ipetkov/crane)**, a composable Rust build system for Nix that:

1. **Automatically handles git dependencies** - No manual `outputHashes` required
2. **Supports complex workspaces** - Better workspace member discovery
3. **Provides incremental builds** - Caches dependencies separately
4. **Includes built-in checks** - Clippy and fmt checks for CI

## Implementation

### Key Changes to flake.nix

#### 1. Added Crane and Rust Overlay Inputs

```nix
inputs = {
  nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  crane.url = "github:ipetkov/crane";
  rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };
};
```

#### 2. Set Up Crane with Rust Toolchain

```nix
overlays = [ (import rust-overlay) ];
pkgs = import nixpkgs { inherit system overlays; };

craneLib = (crane.mkLib pkgs).overrideToolchain
  (p: p.rust-bin.stable.latest.default);
```

#### 3. Configured Source Filtering

**Critical**: Must include `res/` directory for resources:

```nix
src = pkgs.lib.cleanSourceWith {
  src = ./.;
  filter = path: type:
    let
      baseName = baseNameOf (toString path);
      isRes = (type == "directory") && (baseName == "res");
      isInRes = pkgs.lib.hasInfix "/res/" path;
    in
      isRes ||
      isInRes ||
      (craneLib.filterCargoSources path type);
};
```

#### 4. Built Dependencies Separately (Caching)

```nix
commonArgs = {
  inherit src;
  strictDeps = true;
  nativeBuildInputs = [ pkg-config just ];
  buildInputs = [ libxkbcommon wayland expat fontconfig freetype ];
  cargoExtraArgs = "-p cosmic-ext-applet-package-updater";
};

cargoArtifacts = craneLib.buildDepsOnly commonArgs;
```

#### 5. Built the Package

```nix
package = craneLib.buildPackage (commonArgs // {
  inherit cargoArtifacts;
  pname = "cosmic-ext-applet-package-updater";
  version = "1.0.0";

  postInstall = ''
    # Copy resources from $src
    cp $src/res/com.github.cosmic_ext.PackageUpdater.desktop $out/share/applications/
    # ... icons, metainfo ...
  '';
});
```

## Build Results

✅ **Successful build** with:
- Binary: `result/bin/cosmic-ext-applet-package-updater`
- Desktop file: `result/share/applications/`
- Icons: `result/share/icons/hicolor/`
- Metainfo: `result/share/metainfo/`

## Advantages Over buildRustPackage

| Feature | buildRustPackage | Crane |
|---------|------------------|-------|
| Git dependencies | Manual outputHashes (40+ hashes) | Automatic |
| Workspace support | Limited (cargo metadata) | Excellent |
| Build caching | Monolithic | Layered (deps separate) |
| Incremental builds | No | Yes |
| CI checks | Manual setup | Built-in (clippy, fmt) |
| Learning curve | Lower | Medium |

## Usage

### Local Build

```bash
nix build
./result/bin/cosmic-ext-applet-package-updater
```

### Install to Profile

```bash
nix profile install
```

### Run Directly

```bash
nix run
```

### Development

```bash
nix develop
just build-release
```

### System Integration

```nix
# In your NixOS configuration:
inputs.cosmic-package-updater.url = "github:olafkfreund/cosmic-applet-package-updater";

# Then use:
environment.systemPackages = [
  inputs.cosmic-package-updater.packages.${pkgs.system}.default
];
```

## Additional Features

### CI Checks

```bash
nix flake check   # Runs all checks
nix build .#checks.x86_64-linux.clippy  # Just clippy
nix build .#checks.x86_64-linux.fmt     # Just format check
```

### Overlay Support

```nix
nixpkgs.overlays = [
  inputs.cosmic-package-updater.overlays.default
];

environment.systemPackages = with pkgs; [
  cosmic-ext-applet-package-updater
];
```

## Lessons Learned

### 1. Source Filtering is Critical

Crane aggressively filters sources by default. For applications with resource files (desktop files, icons, etc.), you **must** explicitly include them:

```nix
filter = path: type:
  isResourceDir || isCargoSource;
```

### 2. Use $src in postInstall

Don't use relative paths in `postInstall`:

```bash
# ❌ Wrong
cp res/file.desktop $out/share/

# ✅ Correct
cp $src/res/file.desktop $out/share/
```

### 3. Workspace Member Selection

Use `cargoExtraArgs` to build specific workspace members:

```nix
cargoExtraArgs = "-p cosmic-ext-applet-package-updater";
```

### 4. System Libraries

COSMIC apps need these system libraries:

```nix
buildInputs = [
  libxkbcommon
  wayland
  expat
  fontconfig
  freetype
];
```

## Comparison with Other COSMIC Projects

### nixos-cosmic Approach

The official [nixos-cosmic](https://github.com/lilyinstarlight/nixos-cosmic) repository uses:
- `buildRustPackage` with extensive `outputHashes`
- Custom `libcosmicAppHook` for COSMIC-specific builds
- Manual hash management for all git dependencies

**Why we chose crane instead**:
- Simpler to maintain (no manual hash updates)
- Better for rapid development (automatic git dep handling)
- More suitable for individual apps vs. full COSMIC system

### cosmic-launcher Approach

The [cosmic-launcher](https://github.com/pop-os/cosmic-launcher) uses:
- Crane with fenix for Rust toolchain
- nix-filter for advanced source filtering
- Similar pattern to our solution

## Troubleshooting

### Build fails with "Cannot find path for crate"

This was the original issue. Solution: Use crane, not buildRustPackage.

### Resources not copied in postInstall

Fix the source filter to include `res/` directory and use `$src/res/` in postInstall.

### Crane warnings about version/name

Fixed by adding to root Cargo.toml:

```toml
[workspace.package]
version = "1.0.0"

[workspace.metadata.crane]
name = "cosmic-ext-applet-package-updater"
```

### Build is slow

First build is slow (fetches all deps). Subsequent builds are fast due to caching:
- `cargoArtifacts` layer caches dependencies
- Only rebuild when dependencies change

## Future Improvements

1. **Add to nixos-cosmic**: Contribute this package to the official nixos-cosmic flake
2. **Upstream to nixpkgs**: Package for nixpkgs when stable
3. **Binary cache**: Set up cachix for pre-built binaries
4. **Cross-compilation**: Use crane's cross-compilation support for other platforms

## References

- **Crane Documentation**: https://crane.dev
- **Crane GitHub**: https://github.com/ipetkov/crane
- **nixos-cosmic Flake**: https://github.com/lilyinstarlight/nixos-cosmic
- **NixOS Rust Guide**: https://nixos.wiki/wiki/Rust
- **Rust Overlay**: https://github.com/oxalica/rust-overlay

## Credits

Solution developed through research of:
- Crane's workspace examples and documentation
- nixos-cosmic's packaging patterns
- cosmic-launcher's flake implementation
- NixOS community best practices

## Conclusion

**Crane successfully solved the complex git workspace dependency problem**, enabling a pure Nix build without manual hash management. This makes the project more maintainable and easier to integrate into NixOS systems.

**Build command**: `nix build` ✅
**It just works!** 🎉
