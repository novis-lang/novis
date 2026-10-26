The address that `nvs debug` serves the debug viewer on. It is an IP address and a port. The default
is `127.0.0.1:8137`.

Open the address in a browser to see the viewer. `nvs debug --listen <address>` uses another address
for one run. If the port is in use, `nvs debug` stops with an error.

On a loopback address, such as `127.0.0.1`, the viewer uses plain HTTP. Only a browser on the same
computer can open it. On any other address, the viewer uses HTTPS and needs a token. `nvs debug`
creates a certificate, a key and a token in `debug/` in the data folder. Then it prints a link that
contains the token. Open that link once, and the browser keeps the token in a cookie.

`[debug] tls_cert_file`, `tls_key_file` and `token_file` name your own files instead. `nvs debug`
reads this setting when it starts. A program cannot change it.
