[
    "dev-hugepages.mount"
    "bluetooth.target"
    "cryptsetup.target" "remote-cryptsetup.target" "cryptsetup-pre.target"
    "swap.target"
    "tpm2.target"
    "systemd-tmpfiles-clean.timer"
    "systemd-ask-password-console.path" "systemd-ask-password-console.service"
    "systemd-ask-password-wall.service" "systemd-ask-password-wall.path"
    "sys-kernel-config.mount"

    "systemd-oomd.service"
    "sleep.target" "hybrid-sleep.target" "systemd-hybrid-sleep.service" "systemd-hibernate.service" "systemd-hibernate-clear.service" "systemd-suspend-then-hibernate.service" "suspend.target" "suspend-then-hibernate.target" "sleep.target.wants"
    "remote-fs-pre.service" "remote-fs-pre.target" "remote-fs.service" "remote-fs.target"
    "rpcbind.service" "rpcbind.target"
    "systemd-update-done.service"
    "system-update.target" "system-update-pre.target" "system-update-cleanup.service"

    "container-getty.service"
    "container-getty@.service"
    "container@.service"
    "systemd-nspawn@.service"

    "systemd-machine-id-commit.service" "machine.slice" "machines.target" "systemd-machined.service" "dbus-org.freedesktop.machine1.service"

    "dbus-org.freedesktop.portable1.service"
    "systemd-portabled.service"

    "systemd-exit.service"
    "systemd-fsck-root.service" "systemd-fsck@.service"
    "systemd-localed.service" "dbus-org.freedesktop.locale1.service"
    "ctrl-alt-del.target"

    "sound.target" "smartcard.target" "systemd-backlight@.service"
]
