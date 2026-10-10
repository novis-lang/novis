The error a directory operation throws when it fails.

Every method of a directory connection throws `Core\Ldap\LdapError` when the operation does not
succeed. A login is rejected, an entry does not exist, the account has no permission, or the server
does not answer. The error has three properties. `kind` is a `Core\Ldap\ErrorKind` case that says
why the operation failed. `code` is the number the server sent, or `null` when the server sent no
answer. `message` is a sentence for your log.

`LdapError` is a `RuntimeError`, so a `catch (RuntimeError $error)` block also catches it.

**Good to know:** the message never contains a password. A wrong password and a user that does not
exist give the same `kind` and the same message.
