When `Core\Net` lands, a program will want to connect to a socket path directly, and the restriction
in `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it` has to give. The shape that
answers it is a **separate** capability, `net.local`, asked at `Scope::Path` under
`rule:security/path-scope-canonicalise-then-prefix`, whose entries are absolute paths or directory
prefixes and which carries no address policy because there is no address:

```toml
[app.capabilities.net]
local = ["/run/redis.sock", "/var/run/mysqld/"]
```

It is named and **not added to the roster**, because the roster is closed on purpose
(`rule:security/capability-roster-is-closed`) and a capability with no caller is a row that cannot be
exercised. Naming it buys the thing an unwritten decision usually fails to buy: whoever writes
`Core\Net` finds the answer rather than the question.

It is deliberately not `net.connect` widened to admit paths. That grant's whole documented character
is that it carries an address policy (`rule:security/net-address-policy`), and entries of two kinds
under one name — hosts governed by a table, paths governed by nothing — is one name covering two
guarantees.
