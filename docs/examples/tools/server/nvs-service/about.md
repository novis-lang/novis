`nvs service` installs a server as a service of the operating system, which starts it at every boot.

`nvs service install` stores one `nvs` command under a name that you choose. You write the command
after `--`, and it must be `nvs serve` or `nvs run`. Every path in it must be a full path, and
`--config` must name the configuration file. `start`, `stop` and `status` control the installed
service. After `stop`, each request that is running still gets its full response. `unit` prints
what `install` stores and changes nothing. `uninstall` removes the service.

The service manager is systemd on Linux and the Service Control Manager on Windows. You install a
service as root on Linux, and as an administrator on Windows.

**Good to know:** a service has no terminal. Set `[log] target` in `nvs.toml` to a file, and the
log of the server is written to that file.

**The example below** reads the log file that a service uses.
