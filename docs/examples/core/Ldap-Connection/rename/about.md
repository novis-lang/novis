Gives an entry in the directory a new DN. It returns nothing.

The new DN can have the same parent as the old one. Then the entry only gets a new name, such as a
unit called `Sales Team` that becomes `Sales`. The new DN can also have another parent. Then the
entry moves there, such as a person who changes to another team.

The entry keeps everything else. Its attributes and its GUID stay the same, and so does the login
name of a user account. Entries below a unit move with it.

The connection must log in as an account that may change the entry. When another entry already has
the new DN, `rename` throws `Core\Ldap\LdapError` with `$kind` set to `AlreadyExists`, and nothing
changes.

**Good to know:** a program that saved the old DN cannot find the entry with it any more. Save the
GUID instead, because it never changes.
