The forward proxy that every outbound HTTP call uses. Only the configuration file can set it.

When the `[http.client.proxy]` block is in the configuration file, calls use that proxy. When the
block is missing, no proxy is used. There is no proxy option for a single call. Novis also does not
read `HTTP_PROXY`, `HTTPS_PROXY` or `NO_PROXY` from the environment.

`url` is the proxy. `resolve` says whether Novis or the proxy looks up the destination host.
`bypass` lists the hosts that are reached without the proxy. `username`, `password` and
`password_file` are the login for the proxy.

Novis checks the address of every outbound call before it connects. A proxy makes the connection
for Novis, so the person who runs the server must choose it. A program that could choose a proxy
could avoid that check.

The example reads the keys and tries to set each one. `Core\Config::set` returns `false` every
time.
