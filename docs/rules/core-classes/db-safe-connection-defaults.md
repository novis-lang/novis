Three defaults close three common driver holes, and none of the three is configurable to the unsafe
value.

**`LOCAL INFILE` is off**, with no option to enable it — a server that asks the client to send it a
file gets nothing. **TLS defaults to `VerifyFull`** on a TCP connection, never
`sslmode=prefer`, which silently connects in plaintext when the server says so, and a settings
object naming a weaker mode does not compile. **The one exception is a loopback address**
(`127.0.0.0/8`, `::1`) on PostgreSQL, MySQL and MariaDB: a server there that offers no TLS is used
in the clear, because those bytes never leave the machine, as a Unix-domain socket's do not. A
loopback server that offers TLS is still upgraded and verified. The loopback address still passes
the address table and `net.internal` (`rule:security/net-address-policy`), unless an operator's
`[db.<name>]` block wrote it. SQL Server is unchanged. **The connection charset is forced to UTF-8**
(`utf8mb4` on MySQL and MariaDB), so text columns arrive as valid UTF-8 and
`rule:types/string-is-utf8`'s guarantee holds by construction rather than by hope.

The cost is one connection option a deployment cannot turn down, which is the point: an unsafe
default that can be restored is an unsafe default a misconfigured deployment still has.

**Declined:**
- **A setting that turns TLS off for a non-loopback address**, such as a database on a private
  container network. The traffic crosses a network, and the address table already bounds which
  addresses a program reaches; a plaintext exception there is `sslmode=prefer` with an operator's
  signature.
