`nvs ctl` sends a request to a server that is already running, and prints the answer.

It has three requests. `nvs ctl reload` makes the server read its configuration files again and
apply the changes at once. It prints each key that it applied, and each key that needs a restart.
`nvs ctl config` prints the configuration that the server is running with, and the file that set
each key. `nvs ctl status` prints how many requests are running and whether the server is stopping.

`nvs ctl` connects to the control socket of the server, which `[control] socket` in `nvs.toml`
names. Only the account that runs the server can use this socket. It has no password and no network
address.

**Good to know:** when a configuration file has an error, `nvs ctl reload` prints the error and the
server keeps the configuration it has. A program can read `[control] socket`, but it cannot change
it.

**The example below** prints the name of the control socket and the command that reloads the
server.
