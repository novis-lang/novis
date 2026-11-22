Returns the account the connection is logged in as, as the server reports it.

The result is a `tainted string`, because the server sends it. It has one of two forms. It starts
with `dn:` and then the DN of the account, or with `u:` and then a login name. A DN (distinguished
name) is the full path of an entry in the directory. Active Directory and Samba return the login name with the
domain in front, such as `u:EXAMPLE\Reader`. A connection that did not log in returns an empty
string.

Use it to check that a program uses the account you expect, for example when a job starts or in a
health check. Each call sends one request to the server.

**Good to know:** a closed connection throws `LogicError`. When the server does not answer or
returns an error, `whoami` throws `Core\Ldap\LdapError`.
