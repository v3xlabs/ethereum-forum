{ pkgs ? import <nixpkgs> {
    overlays = [
      (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  }
}:

pkgs.mkShell {
  buildInputs = [
    pkgs.rust-bin.stable.latest.default
    pkgs.bacon
    pkgs.pkg-config
    pkgs.openssl
  ];

  shellHook = ''
    echo "Rust development shell loaded."
    echo "Rust version: $(rustc --version)"
  '';
}
