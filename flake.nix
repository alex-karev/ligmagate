{
  description = "OpenAI API-compatible LLM gateway";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
    naersk.url = "github:nix-community/naersk/master";
    utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    self,
    nixpkgs,
    utils,
    naersk,
  }:
    utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
      naersk-lib = pkgs.callPackage naersk {};
    in {
      #defaultPackage = naersk-lib.buildPackage ./.;
      devShell = pkgs.mkShell {
        name = "ratman";
        buildInputs = with pkgs; [
          cargo
          rustc
          rustfmt
          rust-analyzer
          rustPackages.clippy
          pkg-config
        ];
        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
      };
    });
}
