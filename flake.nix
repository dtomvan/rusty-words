{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  nixConfig = {
    extra-substituters = [ "https://crane.cachix.org" ];
    extra-trusted-public-keys = [ "crane.cachix.org-1:8Scfpmn9w+hGdXH/Q9tTLiYAE/2dnJYRJP7kl80GuRk=" ];
  };

  outputs = {
    nixpkgs,
    crane,
    fenix,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      craneLib = (crane.mkLib pkgs).overrideToolchain (p:
        fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-dQWHbEEQOreGVxzawb8LYbstYd1IBpdBtY2ELj0ahB4=";
        });
      src = craneLib.cleanCargoSource ./.;

      commonArgs = {
        inherit src;
		cargoLock = ./Cargo.lock;
        strictDeps = true;
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      rwds-cli = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
		  pname = "rwds-cli";
		  version = "0.1.0";
		  cargoExtraArgs = "--bin rwds-cli";
        });
    in {
      packages = {
        default = rwds-cli;
		genericLinux = rwds-cli.overrideAttrs(final: prev: {
			nativeBuildInputs = prev.nativeBuildInputs ++ [pkgs.patchelf];
			postInstall = ''
			patchelf --set-interpreter /lib/ld-linux.so.2 $out/bin/rwds-cli
			'';
		});
      };

      apps.default = flake-utils.lib.mkApp {
        drv = rwds-cli;
      };

      devShells.default = craneLib.devShell {
        packages = [];
      };
    });
}
