A simple bind with an empty password is refused by the client before a byte is sent, as the error kind `InvalidCredentials`. RFC 4513 § 5.1.2 makes a name with an empty password an *unauthenticated* bind, and Active Directory answers it as a successful anonymous bind, so a login form that passes an empty field through would sign anybody in.

The refusal is the client's, never the server's: Samba answers the same bind `invalidCredentials` and
AD answers it success, and the test asserts nothing reached the wire. It applies to `authenticate` and
to every bind a connection makes. A configuration block or `Ldap\Settings` with a `user` and an empty
password is refused when it is read. An anonymous bind is a block with no `user` at all, written on
purpose, never an empty password. The kind is `InvalidCredentials`, the same as a wrong password, so an
empty field tells a caller nothing a wrong one does not.
