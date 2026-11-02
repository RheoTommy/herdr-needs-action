{
  description = "herdr plugin that marks agent panes as needing action until you handle them";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Where the checks and the dev shell take herdr from: any flake with
    # herdr in packages or legacyPackages, nixpkgs included, so that a user
    # can make it follow the flake they install herdr from and check against
    # that herdr. llm-agents by default rather than nixpkgs, since it
    # packages each herdr release within a day. The package itself does not
    # depend on herdr: it calls the herdr that runs it.
    herdr-pkgs.url = "github:numtide/llm-agents.nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      herdr-pkgs,
    }:
    let
      herdrFor = system: (herdr-pkgs.packages or herdr-pkgs.legacyPackages).${system}.herdr;
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ] (
          system: f system nixpkgs.legacyPackages.${system}
        );
    in
    {
      packages = forAllSystems (
        _: pkgs: {
          default = pkgs.callPackage ./nix/package.nix { };
        }
      );

      checks = forAllSystems (
        system: pkgs: {
          package = self.packages.${system}.default;
          integration = pkgs.callPackage ./nix/integration.nix {
            herdr = herdrFor system;
            plugin = self.packages.${system}.default;
          };
        }
      );

      devShells = forAllSystems (
        system: pkgs: {
          default = pkgs.mkShell {
            packages = [
              pkgs.cargo
              pkgs.rustc
              pkgs.clippy
              pkgs.rustfmt
              pkgs.rust-analyzer
              pkgs.jq
              (herdrFor system)
            ];
          };
        }
      );

      formatter = forAllSystems (_: pkgs: pkgs.nixfmt-tree);
    };
}
