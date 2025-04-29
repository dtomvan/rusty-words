{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";
    treefmt-nix.url = "github:numtide/treefmt-nix";
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
      self,
      nixpkgs,
      flake-utils,
      treefmt-nix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        rwds-cli = pkgs.callPackage ./default.nix { };

        treefmt = treefmt-nix.lib.evalModule pkgs ./treefmt.nix;
      in
      {
        packages = {
          default = rwds-cli;
          genericLinux = rwds-cli.overrideAttrs (
            final: prev: {
              nativeBuildInputs = prev.nativeBuildInputs ++ [ pkgs.patchelf ];
              fixupPhase = ''
                runHook preFixup

                find $out -type f -executable \
                  -exec patchelf \
                    --set-interpreter \
                    /lib/ld-linux.so.2 \
                    {} \;

                runHook postFixup
              '';
              doInstallCheck = false; # at this point the binary doesn't work with Nix anymore
            }
          );
        };

        apps.default = flake-utils.lib.mkApp {
          drv = rwds-cli;
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            clippy
            rust-analyzer
          ];
        };

        formatter = treefmt.config.build.wrapper;
        checks.formatting = treefmt.config.build.check self;
      }
    );
}
