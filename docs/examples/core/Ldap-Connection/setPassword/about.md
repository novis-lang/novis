Sets a new password for a user account. It returns nothing.

This is what an administrator or a help desk does. The old password is not needed, and the account
can log in with the new password at once. A user who changes their own password uses
`changePassword` instead, which checks the old password.

The connection must log in as an account that may reset passwords, such as an administrator.
The connection must also use TLS, because the server only accepts a password over an encrypted
connection. On a connection without TLS, `setPassword` throws an error and sends nothing.

The domain checks the new password against its rules, such as the minimum length. A password that
breaks a rule throws `Core\Ldap\LdapError` with `$kind` set to `PasswordPolicy`.

**Good to know:** the password is a `secret` string. A program cannot print it or write it to a
log. After a reset, you can set `pwdLastSet` to `0`. The user must then choose a new password at
the next login.
