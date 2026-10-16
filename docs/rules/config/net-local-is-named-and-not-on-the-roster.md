A program that supplies a socket path — connecting to one or binding one — needs `net.local`, a
capability **separate** from `net.connect`, asked at `Scope::Path` under
`rule:security/path-scope-canonicalise-then-prefix`, whose entries are absolute paths or directory
prefixes and which carries no address policy because there is no address:

```toml
[app.capabilities.net]
local = ["/run/redis.sock", "/var/run/mysqld/"]
```

**It governs both ends of a path.** Binding one is granted the same way as connecting to one: a
program that may create a socket at a path is a program that whatever else on the host finds it may
speak to, and a path the operator did not name is one they cannot have intended.

It is deliberately not `net.connect` widened to admit paths. That grant's whole documented character
is that it carries an address policy (`rule:security/net-address-policy`), and entries of two kinds
under one name — hosts governed by a table, paths governed by nothing — is one name covering two
guarantees. The general form of that separation is
`rule:security/net-listen-is-a-separate-grant-from-net-connect`: the grant follows what the program is
doing rather than the transport it does it over.

`rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it` is unaffected and stays exactly
as strict. `Core\Net::connect` still may not take a path, because it is not the member that takes one —
a host and a path are separately-typed arguments to separate members, which is
`rule:security/a-path-is-not-a-url` holding by construction rather than by a check.
