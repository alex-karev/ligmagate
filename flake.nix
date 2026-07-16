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
        name = "ollmo";
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
      nixosModules = let
        module = {
          config,
          lib,
          pkgs,
          ...
        }: let
          cfg = config.services.ollmo;
        in
          with lib; {
            options.services.ollmo = {
              enable = mkEnableOption "ollmo";
              configPath = mkOption {
                type = types.path;
                default = "/etc/ollmo";
                description = "Path to the ollmo configs.";
              };
            };

            config = mkIf cfg.enable {
              systemd.services.ollmo = {
                description = "OpenAI API-compatible LLM gateway";
                wantedBy = ["multi-user.target"];
                serviceConfig = {
                  Type = "simple";
                  ExecStart = "${self.packages.${pkgs.system}.default}/bin/ollmo -c ${cfg.configPath}";
                  Restart = "on-failure";
                };
              };
            };
          };
      in {
        default = module;
        ollmo = module;
      };
    };
}
