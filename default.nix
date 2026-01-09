{ lib
, rustPlatform
, pkg-config
, libxkbcommon
, wayland
, just
}:

rustPlatform.buildRustPackage {
  pname = "cosmic-ext-applet-package-updater";
  version = "1.0.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    just
  ];

  buildInputs = [
    libxkbcommon
    wayland
  ];

  # Build only the package-updater workspace member
  buildAndTestSubdir = "package-updater";

  postInstall = ''
    mkdir -p $out/share/applications
    mkdir -p $out/share/metainfo
    mkdir -p $out/share/icons/hicolor

    cp res/com.github.cosmic_ext.PackageUpdater.desktop $out/share/applications/
    cp res/com.github.cosmic_ext.PackageUpdater.metainfo.xml $out/share/metainfo/

    for size in $(ls res/icons/hicolor); do
      mkdir -p $out/share/icons/hicolor/$size/apps
      cp res/icons/hicolor/$size/apps/com.github.cosmic_ext.PackageUpdater.svg \
         $out/share/icons/hicolor/$size/apps/
    done
  '';

  meta = with lib; {
    description = "Package update notifier applet for COSMIC desktop";
    homepage = "https://github.com/olafkfreund/cosmic-applet-package-updater";
    license = licenses.gpl3Only;
    maintainers = [ ];
    platforms = platforms.linux;
  };
}
