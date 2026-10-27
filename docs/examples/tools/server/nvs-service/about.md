`nvs service` installs a server as a service of the operating system, which starts it at every boot.

`nvs service install` stores one `nvs` command under a name you choose. You write the command after
`--`, and it must be `nvs serve` or `nvs run`. Every path in it must be a full path, and `--config`
must name the configuration file. `start`, `stop` and `status` control the service. If the service
is not running, `status` also prints the end of its last run. `unit` prints what `install` stores.
`uninstall` removes the service.

The service manager is systemd on Linux and the Service Control Manager on Windows. You install a
service as root on Linux, and as an administrator on Windows.

`install` and `status` check the owner of each `--config` file for the service's account. This is `--account`, or else root on Linux and `LocalSystem` on Windows. If that account would not accept a
file, `install` stops with `E0607` and changes nothing. `status` prints the error and exits with `1`.

**Good to know:** a service has no terminal. Set `[log] target` in `nvs.toml` to the file
for the server's log.

**The example below** reads the log file that a service uses.
