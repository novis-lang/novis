Returns what kind of account an Active Directory entry is, as a `Core\Ldap\Ad\AccountType`. For
example, a person is `AccountType::User`, a computer is `AccountType::Machine` and a security group
is `AccountType::Group`.

Active Directory stores the kind as a number in `sAMAccountType`. Select that attribute in the
search. `accountType` returns the case of the enum for that number, so you can use it in a `match`.

If the entry has no `sAMAccountType`, the result is `null`. An organizational unit has none. If the
value is not a number that Active Directory uses for this attribute, `accountType` throws a
`LogicError` that names the attribute.
