Checks a user's login and password. It returns nothing when they are correct, and throws an error
when they are not.

You give it the login and the password. The login can be written in three ways: the user's DN,
`name@example.test`, or `EXAMPLE\name`. A login from a form is `tainted`, and `authenticate`
accepts it as it is. The password is a `secret`, so it never shows up in an error message or a log.

`authenticate` opens a new connection to the same server and logs in as the user there. Then it
closes that connection. The connection you call it on stays logged in as its own account, so the
next search still runs with the same rights.

If the login fails, `authenticate` throws `Core\Ldap\LdapError`. Its `$kind` says why. A wrong
password and a user that does not exist both give `InvalidCredentials`, so nobody can use the error
to find out which users exist. Active Directory also gives reasons such as `AccountDisabled`,
`AccountLocked`, `PasswordExpired` and `MustChangePassword`.

**Good to know:** an empty login or an empty password is `InvalidCredentials`, and nothing is sent
to the server. Each call opens one new connection, so a login takes one more TLS handshake than a
search does.
