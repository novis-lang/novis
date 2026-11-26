Settings for every outbound TLS (encrypted) connection: HTTP calls, mail, LDAP, databases and the
shared cache (`Core\Cache::shared()`).

`roots` is the list of trusted certificate authorities (CAs). When it is not set, Novis uses the
Mozilla set that ships with it. You can add your own CA to that set, or list only your own files.
A `[mail.<name>]`, `[ldap.<name>]` or `[db.<name>]` block with its own `tls_ca_file` uses that file
and not `roots`. `min_version` is the lowest TLS version allowed, `1.2` or `1.3`. The default is
`1.2`. `keylog` is a file of the keys of each connection. A network tool can decrypt your own
traffic with it. `keylog` is not allowed when `[mode] default` is `production`.

Only the person who runs the server sets these keys. A program cannot change them.

When a certificate is signed by a CA that Novis does not trust, the error names the setting to
change. When a certificate is for another host name, the error lists the names it is for.

**Good to know:** a running server applies a change when you save `nvs.toml`. If a file in `roots`
contains no certificate, the old settings stay in use. The server writes a warning to its log.
