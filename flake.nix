{
  nixConfig = {
    allow-unsafe-native-code-during-evaluation = true;
  };
  inputs = {
    # <frameworks>
    nixpkgs.url = "github:nixos/nixpkgs?rev=dad564433178067be1fbdfcce23b546254b6d641";

    flake-parts.url = "github:hercules-ci/flake-parts";

    # <tools>
    devenv.url = "github:cachix/devenv?rev=f318d27a4637aff765a378106d82dfded124c3b3"; # https://github.com/cachix/devenv/issues/1513
    devenv.inputs.nixpkgs.follows = "nixpkgs";

    nix-filter.url = "github:numtide/nix-filter";
  };

  outputs = {self, ...} @ inputs: let
    lib = inputs.nixpkgs.lib;
  in
    with builtins;
    with inputs.nixpkgs.lib;
    # Variable meaning:
    # self - flake itself
    # config - current module value
    # pkgs - imported nixpkgs
    # lib - imported lib fro nixpkgs
      inputs.flake-parts.lib.mkFlake
      {
        inherit inputs;
        specialArgs = {inherit lib usr;};
      }
      ({withSystem, ...}: {
        imports = with inputs; [devenv.flakeModule ./dev/shell.nix];
        systems = ["x86_64-linux"];
        perSystem = {self', inputs', system, pkgs, ...}: let
          rustToolchain = with inputs'.fenix.packages; combine [latest.rustc latest.rust-src latest.cargo];
        in {
          _module.args = {
            pkgs = import inputs.nixpkgs {
              inherit system;
              config.allowUnfree = true;
            };
            inherit rustToolchain;
            commonDevshellModule = {config, ...}: {
              _module.args = rec {
                inherit (config.devenv) root state profile;
                fromRoot = source: ''
                  pushd ${root}
                    ${source}
                  popd'';
              };
              # default
              containers = mkForce {};
            };
          };
        };
      });
}
