The certificate file that `nvs debug` uses for HTTPS. There is no default.

`nvs debug` uses HTTPS when `[debug] listen` is an address that is not loopback. Without this
setting, `nvs debug` creates a certificate in `debug/` in the data folder and uses it. That
certificate is self-signed, so the browser shows a warning the first time. Set this to a certificate
that the browser already trusts.

`[debug] tls_key_file` names the private key of the same certificate. `nvs debug` uses your files
only when both of them exist. If a file does not exist, `nvs debug` prints a warning and uses the
certificate it created.

`nvs debug` reads this setting when it starts. A program cannot change it.
