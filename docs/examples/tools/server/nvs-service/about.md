`nvs service` installs a server as a service of the operating system, which starts it at every boot.

`nvs service install` stores one `nvs` command under a name you choose. You write the command after
`--`, and it must be `nvs serve` or `nvs run`. Every path in it must be a full path. `start`, `stop`
and `status` control the service. `unit` prints what `install` stores. `uninstall` removes the
service.

The service manager is systemd on Linux and the Service Control Manager on Windows. You install a
service as root on Linux, and as an administrator on Windows.

`install` and `status` check the owner of each `--config` file for the service's account. If that
account would not accept a file, `install` stops with `E0607` and changes nothing.

A port from 1 to 1023 is a privileged port. For a server on one, `install` on Linux also writes a
systemd socket unit. systemd listens for the server, so the service needs no extra permission.

**Good to know:** a service has no terminal. Set `[log] target` in `nvs.toml` to the file
for the server's log.

**The examples below** read the log file that a service uses, and show which ports are privileged.
