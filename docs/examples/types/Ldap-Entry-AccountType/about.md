Says what kind of object an Active Directory entry is.

`Core\Ldap\Entry::AccountType` is declared in the class `Core\Ldap\Entry`. The method `accountType`
returns it.

Active Directory keeps people, computers and groups in the same tree. Each of them has an
attribute, `sAMAccountType`, that says which kind it is. `Core\Ldap\Entry::accountType()` reads that
attribute and returns one of these cases. `User` is a user account, `Machine` is a computer, and `Group` and `Alias` are groups. The result is `null` when the entry has no such
attribute, for example an organizational unit.

**Good to know:** the search must select `sAMAccountType`, or `accountType()` returns `null`.
