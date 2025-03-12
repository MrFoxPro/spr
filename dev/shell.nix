{inputs, lib, ...} @ rootArgs:
with builtins;
with lib; {
  imports = [./vm.nix];
  perSystem = {system, self', inputs', pkgs, config, lib, commonDevshellModule, ... } @ systemArgs: {
    devenv.shells.default = {config, fromRoot, ...} @ devenvArgs: let
      shell = config;
      inherit (shell) devenv;
      inherit (devenv) root state profile;
    in {
      imports = [commonDevshellModule];
      env = rec {
        "RUSTFLAGS" = concatStringsSep " " [
          "-Zshare-generics=y"
          "-Clinker=clang"
          "-Clink-arg=--ld-path=ld"
        ];
        "TARGET_CC" = "clang-cl";
        "TARGET_CXX" = TARGET_CC;
        "TARGET_AR" = "llvm-lib";
        "LD_LIBRARY_PATH" = with pkgs; makeLibraryPath [clang libclang libclang.lib libllvm lld];
      };

      # ANSI colors: https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
      enterShell = let
        commands = pipe devenvArgs.config.scripts [
          attrNames
          (groupBy (cmd: elemAt (splitString ":" cmd) 0))
          (mapAttrsToList (group: commands: let
            splitted = pipe commands [
              (sortOn stringLength)
              (map (removePrefix group))
              (concatStringsSep "|")
            ];
          in "$(tput setaf 105)${group}$(tput sgr0)|${splitted}"))
          (intersperse "\n")
          concatStrings
        ];
      in ''
        echo ${root}
        echo "$(tput setaf 105)🖥 Simple Process Runner$(tput sgr0)"
        echo "${commands}" | ${pkgs.unixtools.column}/bin/column --table -W 1 -T 1 -t -s "|"
      '';
      packages = with pkgs; [
        pkg-config
        clang libclang libclang.lib libllvm lld
      ];
      scripts."dev".exec = fromRoot ''
        cargo run -- -c 'ping 1.1.1.1' t1 -cn 'ping 8.8.8.8 -c 6' t2 --notify-vsock 3:9000
      '';
    };
  };
}
