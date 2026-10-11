A simple bind is sent over TLS — LDAPS, or StartTLS on `ldap://` — unless the block or `Ldap\Settings` says `tls = "none"` and the host is on `[capabilities.ldap] cleartext`. The client checks both before the password is written, and without both, `ldap://` runs StartTLS or fails.

Two halves, for `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s reason: the deployment
says *where* this may happen and the code says *here*. `cleartext` is a list of hosts, matched as
`ldap.open` matches them, in the shape of `tls.insecure` and `net.downgrade`, and `true` is not a
spelling it has. Some controllers accept a cleartext bind and the operator decides per host; the cost
is that the password and every answer cross the network readable and changeable. A controller that
refuses one answers `strongerAuthRequired`, which is the kind `EncryptionRequired`. Nothing relaxes
certificate checks beyond what `[capabilities.tls]` already allows per host.

**A simple bind is the only bind**, because every directory this client is for accepts one over TLS.
A deployment whose directory accepts only Kerberos or NTLM cannot use `Core\Ldap`.

**A password is written only over TLS, whatever the grant says.** It is AD's `unicodePwd`, the quoted
string in UTF-16LE, and both writers take `secret` strings (`rule:security/secret-qualifier`).
`setPassword` is a `replace`, an administrator's reset; `changePassword` is a `remove` of the old and
an `add` of the new in one Modify, the user's own change, which AD checks against the password history.

**Declined:**
- **Kerberos, NTLM and every SASL mechanism, `EXTERNAL` included.** Kerberos needs a C GSSAPI library,
  a keytab or a credential cache found through the environment, and clock and DNS state outside the
  request; NTLM is a broken challenge-response over MD4. Both are credential handling on the most
  sensitive surface there is. The question reopens only if a directory this client is for stops
  accepting a simple bind over TLS, and then only for a mechanism that needs no state outside the
  request.
- **A "do not verify" switch, or TLS options set for the process.** A TLS option one connection sets
  would reach another request's connection.
- **RFC 3062's Password Modify operation.** AD does not implement it.
