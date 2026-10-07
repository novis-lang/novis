A simple bind is sent over TLS — LDAPS, or StartTLS on `ldap://` — unless the block or `Ldap\Settings` says `tls = "none"` and the host is on `[capabilities.ldap] cleartext`. The client checks both before the password is written, and without both, `ldap://` runs StartTLS or fails.

Two halves, for `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s reason: the deployment
says *where* this may happen and the code says *here*. `cleartext` is a list of hosts, matched as
`ldap.open` matches them, in the shape of `tls.insecure` and `net.downgrade`, and `true` is not a
spelling it has. Some controllers accept a cleartext bind and the operator decides per host; the cost
is that the password and every answer cross the network readable and changeable. A controller that
refuses one answers `strongerAuthRequired`, which is the kind `EncryptionRequired`. Passwords are
written only over TLS whatever the grant says, and nothing relaxes certificate checks beyond what
`[capabilities.tls]` already allows per host.
