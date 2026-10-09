Changes the password of a user account from the old password to a new one. It returns nothing.

This is what a user does on a "change my password" page. The server checks the old password first.
If it is wrong, the password does not change. An administrator who resets a password without the
old one uses `setPassword` instead.

The connection must use TLS, because the server only accepts a password over an encrypted
connection. On a connection without TLS, `changePassword` throws an error and sends nothing.

The domain checks the new password against its rules. A new password can be too short, the same as
a recent one, or too soon after the last change. Then `changePassword` throws
`Core\Ldap\LdapError` with `$kind` set to `PasswordPolicy`.

**Good to know:** both passwords are `secret` strings. A program cannot print them or write them to
a log. Most domains have a minimum password age, often one day. A password set a moment ago can be
changed at once only when `pwdLastSet` is `0`, which means the user must change it now.
