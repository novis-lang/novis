How much of a server's identity is established before a password is sent to it.

Before a connection hands over a credential it can check the certificate the server presented, and
these four cases name the choices databases usually offer for that. `VerifyFull` is the strongest:
the certificate has to be signed by an authority the machine trusts, and it has to be issued for the
host that was asked for. It is also the only one a connection here ever runs at. The other three
exist so that a program asking for one is told so at the call, in words naming what it asked for,
rather than being handed something stronger without being told.

**Good to know:** what a deployment can change is *whose* certificates are believed — a
`tls_ca_file` on the connection's own configuration block — never whether they are checked at all.
