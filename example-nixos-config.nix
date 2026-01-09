# Example NixOS Configuration Integration
# This shows how to add the COSMIC Package Updater to your system

# In your flake.nix:
{
  description = "My NixOS Configuration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Add the COSMIC Package Updater
    cosmic-package-updater = {
      url = "github:olafkfreund/cosmic-applet-package-updater";
      # Or use a local path for development:
      # url = "path:/home/user/Source/cosmic-applet-package-updater";
    };
  };

  outputs = { self, nixpkgs, cosmic-package-updater, ... }: {
    nixosConfigurations = {
      # Replace with your hostname
      my-nixos-machine = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";

        # Pass inputs to modules
        specialArgs = { inherit cosmic-package-updater; };

        modules = [
          # Your existing configuration
          ./hardware-configuration.nix
          ./configuration.nix

          # Method 1: Use as a simple module (inline)
          ({ pkgs, cosmic-package-updater, ... }: {
            environment.systemPackages = [
              cosmic-package-updater.packages.${pkgs.system}.default
            ];
          })

          # OR Method 2: Use with overlay (in configuration.nix)
          # See below
        ];
      };
    };
  };
}

# === Alternative: In configuration.nix ===
#
# { config, pkgs, cosmic-package-updater, ... }:
#
# {
#   # Method 1: Direct package reference
#   environment.systemPackages = [
#     cosmic-package-updater.packages.${pkgs.system}.default
#   ];
#
#   # OR Method 2: Using overlay
#   nixpkgs.overlays = [
#     cosmic-package-updater.overlays.default
#   ];
#
#   environment.systemPackages = with pkgs; [
#     cosmic-ext-applet-package-updater
#   ];
# }

# === For Home Manager Users ===
#
# In home.nix:
#
# { config, pkgs, cosmic-package-updater, ... }:
#
# {
#   home.packages = [
#     cosmic-package-updater.packages.${pkgs.system}.default
#   ];
# }

# === After Configuration ===
#
# 1. Update flake.lock:
#    nix flake lock --update-input cosmic-package-updater
#
# 2. Rebuild your system:
#    sudo nixos-rebuild switch --flake .#my-nixos-machine
#
# 3. Logout and login to COSMIC
#
# 4. Add applet to panel:
#    Right-click panel → Add Applet → Package Updater
