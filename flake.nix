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
      packages.default = naersk-lib.buildPackage {
        src = ./.;
      };
      devShell = pkgs.mkShell {
        name = "ligmagate";
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
    })
    // {
      nixosModules.default = { config, lib, pkgs, ... }:
        with lib; {
          options.services.ligmagate = {
            enable = mkEnableOption "ligmagate";
            configPath = mkOption {
              type = types.path;
              default = "/etc/ligmagate";
              description = "Path to the ligmagate configs.";
            };
          };

          config = mkIf config.services.ligmagate.enable {
            systemd.services.ligmagate = {
              description = "OpenAI API-compatible LLM gateway";
              wantedBy = ["multi-user.target"];
              serviceConfig = {
                Type = "simple";
                ExecStart = "${self.packages.${pkgs.system}.default}/bin/ligmagate -c ${cfg.configPath}";
                Restart = "on-failure";
              };
            };
          };
        };
    };
}
