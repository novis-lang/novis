A simple bind with an empty password, or with a password that contains a NUL byte, is refused by the client before a byte of the password is sent, as the error kind `InvalidCredentials`. RFC 4513 § 5.1.2 makes a name with an empty password an *unauthenticated* bind, and Active Directory answers it as a successful anonymous bind, so a login form that passes an empty field through would sign anybody in. Samba reads a simple-bind password up to its first NUL, so `secret\0` and `secret\0anything` both log in as the account whose password is `secret`, and a login form would accept a string that is not the password.

The refusal is the client's, never the server's: Samba answers the empty bind `invalidCredentials` and
AD answers it success, and the test asserts nothing reached the wire. Both checks sit in
`nvs_ldap::Connection::bind`, which every simple bind goes through: `authenticate`, `Ldap::connect`'s
block and `Ldap::open`. A configuration block or `Ldap\Settings` with a `user` and an empty password is
refused when it is read. A password with a NUL byte from configuration is refused at bind time, by the
same check. An anonymous bind is a block with no `user` at all, written on purpose, never an empty
password. The kind is `InvalidCredentials`, the same as a wrong password, so an empty field or a NUL
byte tells a caller nothing a wrong password does not. It carries no result code, because no server
sent one.

**Cost.** One scan of the password's bytes per bind, which is nothing beside the bind's round trip. A
password that really contains a NUL byte cannot be used for a simple bind from Novis; no directory
stores one as typed, so no real account is locked out.

**Declined.**

- **Strip the NUL and send the rest.** That sends a password the caller did not pass, and it turns
  `secret\0anything` into `secret`, which is the bug itself.
- **Refuse every control character.** Only NUL cuts the password short on the server. A tab or another
  control character is sent as it is and judged by the server, so a real password that contains one
  still works.
