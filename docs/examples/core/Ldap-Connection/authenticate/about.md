Checks a user's login and password. It returns nothing when they are correct, and throws an error
when they are not.

The login can be the user's DN, `name@example.test` or `EXAMPLE\name`. A login from a form is
`tainted`, and `authenticate` accepts it as it is. The password is a `secret`, so it never shows up
in an error message or a log.

`authenticate` opens a new connection and logs in as the user there. Then it closes that
connection. The connection you call it on keeps its own account and rights.

If the login fails, `authenticate` throws `Core\Ldap\LdapError`, and its `$kind` says why. A wrong
password and a user that does not exist both give `InvalidCredentials`, so the error does not show
which users exist. Active Directory also gives reasons such as `AccountDisabled`, `AccountLocked`
and `PasswordExpired`.

**Good to know:** an empty login or password is `InvalidCredentials`, and nothing is sent to the
server. A password with a zero byte (`\x00`) is also `InvalidCredentials`, and it is not sent. Each
call costs one more TLS handshake.
