{ inputs, lib, withSystem, self, ... } @ rootArgs:
with builtins;
with lib; let
  root = getEnv "DEVENV_ROOT"; state = getEnv "DEVENV_STATE";
  mem = toString 3572;
  debug = false;
  tmpfsRoot = true;
  shares = mapAttrs (k: v: v // {socket = "${state}/vm/${k}.fs.sock";}) {
    "nixstore" = { dir = "/nix/store"; to = "/nix/.ro-store"; fs.options = ["x-initrd.mount"]; fs.neededForBoot = true; };
    "repo" = { dir = root; fs.options = ["suid,exec"]; };
  };
  # for ref https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/virtualisation/nixos-containers.nix
  baseModule = {config, modulesPath, pkgs, ...}: {
    imports = [(modulesPath + "/profiles/minimal.nix")];
    system.stateVersion = "25.05";
    boot.blacklistedKernelModules = ["hid" "usbhid" "hid_generic" "input_leds" "button" "mousedev" "serio"];
    services.fstrim.enable = false;
    systemd = {
      enableEmergencyMode = false;
      oomd.enable = false;
      watchdog.runtimeTime = "2s";
      watchdog.rebootTime = "2s";
      suppressedSystemUnits = import ./sdmin.nix; # disable defaults https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/system/boot/systemd.nix
    };
    services.getty.helpLine = "Type Ctrl-a c to switch to the qemu console and `quit` to stop the VM.";
    environment.loginShellInit = let term = getEnv "TERM"; in ''
      cd '${root}' 2>/dev/null
      ${optionalString (term != "") "export TERM='${term}'"}
    '';
    environment.systemPackages = with pkgs; [strace lsof];
    networking.hostName = "sprvm";
    services.getty.autologinUser = "root";
  };
  vmModule = {config, ...}: {
    boot.kernelParams = ["quiet"];
    boot.initrd.availableKernelModules = ["virtio_blk" "virtio_pci" "virtio_ring" "overlay" "virtiofs"];
    boot.loader.systemd-boot.enable = true;
    boot.loader.efi.canTouchEfiVariables = true;

    # https://github.com/NixOS/nixpkgs/issues/342082
    boot.initrd.preLVMCommands = "export LVM_SUPPRESS_FD_WARNINGS=1";
    fileSystems = let
      fs_shares = mapAttrs' (id: share: {
        name = share.to or share.dir;
        value = { fsType = "virtiofs"; device = id;} // (share.fs or {});
      }) shares;
    in
      {
        "/nix/.rw-store" = {
          fsType = "tmpfs";
          options = ["mode=0755"];
        };
        "/nix/store" = {
          overlay = {
            lowerdir = ["/nix/.ro-store"];
            upperdir = "/nix/.rw-store/upper";
            workdir = "/nix/.rw-store/work";
          };
          neededForBoot = true;
        };
        "/" =
          if tmpfsRoot then {
            fsType = "tmpfs";
            options = ["mode=0755"];
          } else {
            fsType = "ext4";
            device = "/dev/disk/by-id/virtio-root";
            options = ["x-initrd.mount"];
            neededForBoot = true;
          };
      } // fs_shares;
  };
in {
  flake.nixosConfigurations."vm" = withSystem "x86_64-linux" (
    {system, ...}: lib.nixosSystem { inherit system; modules = [baseModule vmModule { environment.interactiveShellInit = "${root}/vm:dev.sh"; }]; }
  );
  perSystem = { system, self', inputs', pkgs, config, lib, commonDevshellModule, rustToolchain, ... } @ systemArgs: {
    devenv.shells.default = {fromRoot, ...} @ devenvArgs: {
      scripts."vm:run".exec = "nix run .#vm --impure";
      scripts."vm:frun".exec = "nix run .#vm --impure --no-substitute --offline --no-use-registries --no-write-lock-file --quiet";
    };
    packages."vm" = let
      nixos = self.nixosConfigurations."vm".config.system;
      closureInfo = pkgs.closureInfo {rootPaths = [nixos.build.toplevel];};
      # https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/virtualisation/qemu-vm.nix
      qemu_cmd = let source = [
        "${pkgs.qemu_kvm}/bin/qemu-system-x86_64 -nodefaults -no-user-config -no-reboot"
        "-name main -machine accel=kvm:tcg -object memory-backend-memfd,id=mem,size=${mem}M,share=on -numa node,memdev=mem"
        "-cpu host -m ${mem} -smp ${toString (exec ["nproc"])} -device virtio-rng-pci"
        (if debug
          then "-chardev stdio,id=stdio,signal=off -serial chardev:stdio"
          else "-serial null -device virtio-serial -chardev stdio,mux=on,id=char0,signal=off -mon chardev=char0,mode=readline -device virtconsole,chardev=char0,nr=0"
        )
        "-kernel ${nixos.build.toplevel}/kernel -initrd ${nixos.build.initialRamdisk}/${nixos.boot.loader.initrdFile}"
        ''-append "$(cat ${nixos.build.toplevel}/kernel-params) init=${nixos.build.toplevel}/init regInfo=${closureInfo}/registration earlyprintk=ttyS0 console=ttyS0"''
        (optionalString (!tmpfsRoot) ''file="$NIX_DISK_IMAGE",id=drive1,if=none,index=1,werror=report,cache=writeback -device virtio-blk-pci,bootindex=1,drive=drive1,serial=root'')
        (mapAttrsToList (id: share: "-chardev socket,id=fs${id},path=${share.socket} -device vhost-user-fs-pci,chardev=fs${id},tag=${id}") shares)
        # "-chardev socket,id=char_com,path=${state}/vm/com.sock,server=on,wait=off -device virtio-serial-pci -device virtserialport,chardev=char_com,name=com" # /dev/virtio-ports/com
        "-device vhost-vsock-pci,id=vhost-vsock0,guest-cid=3" # /dev/vsock
        "-net nic,netdev=user.0,model=virtio -netdev user,id=user.0,net=192.168.0.0/24,host=192.168.0.254,dns=192.168.0.253"
        "-nographic"
        ''"$@"''
      ]; in pipe source [flatten (filter (s: s != "")) (concatStringsSep " \\\n")];
    in
      pkgs.writeShellScriptBin "vm:launch" ''
        set -e
        mkdir -p ${state}/vm
        cleanup() {
          ${concatMapStringsSep "\n" (share: "rm -f ${share.socket} ${share.socket}.pid") (attrValues shares)}
        }
        trap cleanup EXIT SIGINT SIGTERM SIGHUP
        ${concatMapStringsSep "\n" (share: ''[ -f ${share.socket}.pid ] && kill "$(cat ${share.socket}.pid)" &>/dev/null'') (attrValues shares)}
        cleanup

        ${optionalString (!tmpfsRoot) ''
          NIX_DISK_IMAGE=${state}/vm/root.qcow2
          if [ ! -e "$NIX_DISK_IMAGE" ]; then
            temp=$(mktemp)
            ${pkgs.qemu_kvm}/bin/qemu-img create -f raw "$temp" 10G
            ${pkgs.e2fsprogs}/bin/mkfs.ext4 -L root "$temp"
            ${pkgs.qemu_kvm}/bin/qemu-img convert -f raw -o compression_type=zstd,preallocation=falloc -O qcow2 "$temp" "$NIX_DISK_IMAGE"
            rm "$temp"
          fi
        ''}
        ${pipe shares [
          (mapAttrsToList (_: share: "${pkgs.virtiofsd}/bin/virtiofsd --log-level=error --xattr --posix-acl --socket-path=${share.socket} --shared-dir=${share.dir} &"))
          (concatStringsSep "\n")
        ]}
        ${qemu_cmd}
        wait
      '';
  };
}
