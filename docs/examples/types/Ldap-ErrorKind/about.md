Says why a directory operation failed.

When a directory operation fails, the `Core\Ldap\LdapError` your program catches has a `kind`
property. It is one of these cases. `InvalidCredentials` means the login or the password is wrong.
`NoSuchObject` means the entry does not exist. `Unavailable` and `Timeout` mean the server did not
answer in time, so a later try may work. Active Directory gives a separate case for each reason a
login fails, such as `AccountDisabled`, `AccountLocked` and `PasswordExpired`.

Compare `kind` with a case to choose what your program does next.

**Good to know:** a user that does not exist and a wrong password are both `InvalidCredentials`.
Nobody can use a login form to find out which users exist.
