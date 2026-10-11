Says whether a connection to a directory is encrypted.

`Core\Ldap::Tls` is declared in the class `Core\Ldap`. The `tls` field of the settings that
`Core\Ldap::open` takes is one of its cases.

`Tls::Required` is the default. An `ldaps://` URL starts with TLS, and an `ldap://` URL switches to
TLS before it sends the password. When TLS is not possible, the connection throws an error.

`Tls::None` sends the password and every result as plain text. Anybody on the network can read
them and change them. A program may use it only for a host the person who runs the server lists in
`[capabilities.ldap] cleartext` in `nvs.toml`. For every other host, the connection throws a
`Core\Ldap\LdapError` with the kind `EncryptionRequired`.

**Good to know:** use `Tls::None` only for a test server or a server on the same machine.
