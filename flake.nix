{
  description = "rust-modules";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";
    crane.url = "github:ipetkov/crane";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs @ {
    self,
    crane,
    flake-parts,
    nixpkgs,
  }: let
    pname = "rust-modules";
  in
    flake-parts.lib.mkFlake {inherit inputs;} {
      flake.overlays.default = final: prev: {
        ${pname} = final.craneLib.buildPackage {
          src = ./.;
        };
      };

      systems = ["x86_64-linux"];

      perSystem = {
        pkgs,
        self',
        system,
        ...
      }: {
        _module.args.pkgs = import nixpkgs {
          config = {};
          overlays = [
            self.overlays.default
            (final: prev: {craneLib = crane.mkLib prev;})
          ];
          inherit system;
        };

        packages.default = pkgs.${pname};

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
          ];
          inputsFrom = [self'.packages.default];
        };
      };
    };
}
