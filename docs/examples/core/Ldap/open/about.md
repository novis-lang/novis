Opens an LDAP directory at a URL your program gives, with the account your program gives.

Use it when the directory is not known in advance, for example when each customer sets up their own.
`Core\Ldap::open` takes the `url`, the `user` and the `password`, and logs in. It returns a
`Core\Ldap\Connection`, which is closed when the request ends.

The connection is encrypted. `ldaps://` encrypts it from the start. `tls: Tls::None` sends the password
and every answer as plain text, and it works only for a host in `[capabilities.ldap] cleartext`.

The `url` cannot be `tainted` (a value from a request that is not checked yet), because the password
is sent to that host. An empty password throws an error before anything is sent.

**Good to know:** `[capabilities.ldap] open` in `nvs.toml` lists the hosts a program may open. Any
other host throws a `RuntimeError`. A private address, such as `10.0.0.5`, also throws, unless the
operator allows it.
