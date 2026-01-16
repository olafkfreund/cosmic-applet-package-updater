{
  description = "COSMIC Package Updater Applet - NixOS update notifications for COSMIC Desktop";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay }:
    let
      # Overlay to add this package to nixpkgs
      overlay = final: prev: {
        cosmic-ext-applet-package-updater = self.packages.${prev.system}.default;
      };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Crane library for building Rust projects
        craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.stable.latest.default);

        # Source filtering - include res/, policy/ directories and justfile
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            let
              baseName = baseNameOf (toString path);
              isRes = (type == "directory") && (baseName == "res");
              isInRes = pkgs.lib.hasInfix "/res/" path;
              isPolicy = (type == "directory") && (baseName == "policy");
              isInPolicy = pkgs.lib.hasInfix "/policy/" path;
              isJustfile = (type == "regular") && (baseName == "justfile");
            in
              isRes ||
              isInRes ||
              isPolicy ||
              isInPolicy ||
              isJustfile ||
              (craneLib.filterCargoSources path type);
        };

        # Common arguments for all crane builds
        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
            just
            makeWrapper
          ];

          buildInputs = with pkgs; [
            libxkbcommon
            wayland
            wayland-protocols
            expat
            fontconfig
            freetype
          ];

          # Only build the package-updater workspace member
          cargoExtraArgs = "-p cosmic-ext-applet-package-updater";
        };

        # Build dependencies only (for caching)
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual package
        package = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;

          pname = "cosmic-ext-applet-package-updater";
          version = "1.0.0";

          # Skip tests during build - they require file I/O which conflicts with Nix sandbox
          doCheck = false;

          # Use justfile for installation to match COSMIC conventions
          installPhaseCommand = ''
            just --set prefix "$out" --set bin-src "target/release/cosmic-ext-applet-package-updater" install
          '';

          # Wrap binary to ensure Wayland libraries are found at runtime
          postFixup = ''
            wrapProgram $out/bin/cosmic-ext-applet-package-updater \
              --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath [ pkgs.wayland pkgs.libxkbcommon ]}"
          '';

          meta = with pkgs.lib; {
            description = "Package update notifier applet for COSMIC desktop with NixOS support";
            longDescription = ''
              A COSMIC desktop applet that monitors package updates from multiple package managers
              including NixOS (both Flakes and Channels), Pacman, APT, DNF, and Flatpak.

              For NixOS, it provides:
              - Automatic detection of Flakes vs Channels mode
              - Dry-run update checking (shows updates without applying them)
              - Configurable NixOS config path
              - Integration with nixos-rebuild and nix flake commands
            '';
            homepage = "https://github.com/olafkfreund/cosmic-applet-package-updater";
            license = licenses.gpl3Only;
            maintainers = [ ];
            platforms = platforms.linux;
            mainProgram = "cosmic-ext-applet-package-updater";
          };
        });
      in
      {
        packages = {
          default = package;
          cosmic-ext-applet-package-updater = package;
        };

        # Development shell
        devShells.default = pkgs.mkShell {
          inputsFrom = [ package ];

          buildInputs = with pkgs; [
            # Additional development tools
            rust-analyzer
            rustfmt
            clippy
          ];

          shellHook = ''
            echo "🚀 COSMIC Package Updater development environment"
            echo ""
            echo "Available commands:"
            echo "  just build-release  - Build release version"
            echo "  just run           - Run with debug logging"
            echo "  just check         - Run clippy checks"
            echo ""
            echo "Nix commands:"
            echo "  nix build           - Build the package with crane"
            echo "  nix run             - Run the applet directly"
            echo ""
          '';
        };

        # Apps for easy running
        apps.default = {
          type = "app";
          program = "${package}/bin/cosmic-ext-applet-package-updater";
        };

        # Checks for CI
        checks = {
          inherit package;

          # Clippy check
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          # Format check
          fmt = craneLib.cargoFmt {
            inherit src;
          };
        };
      }
    ) // {
      # Make overlay available at top level
      overlays.default = overlay;
    };
}
