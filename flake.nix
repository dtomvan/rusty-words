{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  nixConfig = {
    extra-substituters = [
      "https://nix-community.cachix.org"
    ];

    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  outputs =
    {
      nixpkgs,
      crane,
      fenix,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain fenix.packages.${system}.minimal.toolchain;

        rwds-cli = craneLib.buildPackage {
          src = ./.;
        };
      in
      {
        packages = {
          default = rwds-cli;
          genericLinux = rwds-cli.overrideAttrs (
            final: prev: {
              nativeBuildInputs = prev.nativeBuildInputs ++ [ pkgs.patchelf ];
              postInstall = ''
                patchelf --set-interpreter /lib/ld-linux.so.2 $out/bin/rwds-cli
              '';
            }
          );
        };

        apps.default = flake-utils.lib.mkApp {
          drv = rwds-cli;
        };

        devShells.default = craneLib.devShell {
          packages = [ ];
        };
      }
    );
}
