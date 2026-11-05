{ pkgs }:

let
  browserLibraries = with pkgs; [
    alsa-lib
    atk
    at-spi2-atk
    at-spi2-core
    cairo
    cups
    dbus
    expat
    fontconfig
    freetype
    glib
    libgbm
    libdrm
    libudev-zero
    libxkbcommon
    libxcb
    mesa
    nspr
    nss
    pango
    systemd
    libx11
    libxcomposite
    libxdamage
    libxext
    libxfixes
    libxrandr
  ];
in
pkgs.mkShell {
  packages = with pkgs; [
    bun
    gcc
    pkg-config
    rustup
    wasm-bindgen-cli
  ];

  shellHook = ''
    export CARGO_HOME="''${CARGO_HOME:-$HOME/.cargo}"
    export PATH="$CARGO_HOME/bin:$PATH"
    export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath browserLibraries}:''${LD_LIBRARY_PATH:-}"
  '';
}
