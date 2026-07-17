{
  description = "ethereum-forum devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = [
          "rust-src"
          "llvm-tools"
        ];
      };

      rustfmtNightly = pkgs.rust-bin.nightly.latest.rustfmt;
    in {
      devShells = {
        default = pkgs.mkShell {
          packages = with pkgs; [
            rustfmtNightly
            rustToolchain
            rust-analyzer
            bacon
            just

            nodejs_24
            pnpm_11
          ];

          shellHook = ''
            just
          '';
        };
      };
    });
}
