The private key file that `nvs debug` uses for HTTPS. There is no default.

The key belongs to the certificate that `[debug] tls_cert_file` names. `nvs debug` uses HTTPS when
`[debug] listen` is an address that is not loopback. Without your own files, `nvs debug` creates a
certificate and a key in `debug/` in the data folder and uses them.

`nvs debug` uses your files only when both of them exist. If a file does not exist, `nvs debug`
prints a warning and uses the key it created. Keep the key file readable only by the account that
runs `nvs debug`.

`nvs debug` reads this setting when it starts. A program cannot change it.
