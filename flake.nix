{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.05";
    flake-utils.url = "github:numtide/flake-utils";
    nix-naersk.url = "github:nix-community/naersk";
    nixpkgs-mozilla = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , nix-naersk
    , nixpkgs-mozilla
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          (import nixpkgs-mozilla)
        ];
      };

      toolchain = (pkgs.rustChannelOf {
        rustToolchain = ./rust-toolchain.toml;
        sha256 = "sha256-ks0nMEGGXKrHnfv4Fku+vhQ7gx76ruv6Ij4fKZR3l78=";
      }).rust;

      naersk = pkgs.callPackage nix-naersk {
        cargo = toolchain;
        rustc = toolchain;
      };
    in
    {
      packages = rec {
        default = shipit;
        shipit = import ./package.nix {
          inherit naersk pkgs;
          version = self.rev or "dirty";
        };
      };

      apps.default = flake-utils.lib.mkApp {
        drv = self.packages.${system}.default;
        exePath = "/bin/shipit";
      };

      formatter = pkgs.nixpkgs-fmt;

      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          toolchain
          clang
          cargo-nextest
        ];
      };
    });
}
