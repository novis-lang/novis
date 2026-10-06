Three defaults close three common driver holes, and none of the three is configurable to the unsafe
value.

**`LOCAL INFILE` is off**, with no option to enable it — a server that asks the client to send it a
file gets nothing. **TLS defaults to `VerifyFull`** on a TCP connection, never
`sslmode=prefer`, which silently connects in plaintext when the server says so, and a settings
object naming a weaker mode does not compile. **The connection charset is forced to UTF-8**
(`utf8mb4` on MySQL and MariaDB), so text columns arrive as valid UTF-8 and
`rule:types/string-is-utf8`'s guarantee holds by construction rather than by hope.

The cost is one connection option a deployment cannot turn down, which is the point: an unsafe
default that can be restored is an unsafe default a misconfigured deployment still has.
