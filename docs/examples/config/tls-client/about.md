Settings for every outbound TLS connection: HTTP calls, mail, LDAP, databases and the shared cache.

`roots` is the list of trusted certificate authorities (CAs). When it is not set, Novis uses the
Mozilla set that ships with it. You can add your own CA to that set, or list only your own files.
A block with its own `tls_ca_file` uses that file and not `roots`. `min_version` is `1.2` or `1.3`,
and the default is `1.2`. `keylog` is a file for session secrets, so you can read your own traffic.
It is not allowed on a production host.

Only the person who runs the server sets these keys. A program cannot change them.

When a certificate is signed by a CA that Novis does not trust, the error names the setting to
change. When a certificate is for another host name, the error lists the names it is for.

**Good to know:** a running server applies a change when you save the file. If a file in `roots`
contains no certificate, the old settings stay in use, and the server writes this to its log.
