Returns the type of an Active Directory group as a `Core\Ldap\Ad\GroupType`. Each flag is a function
that returns a `bool`, such as `security()` or `global()`.

Active Directory stores the type as one number in `groupType`. Select that attribute in the search.
`security()` says if the group can give its members access to something. A group where it is `false`
is a distribution group, which is only used for email. `global()`, `domainLocal()` and `universal()`
give the scope of the group.

If the entry has no `groupType`, the result is `null`. A user has none. If the value is not a
number, `groupType` throws a `LogicError` that names the attribute.

**Good to know:** the security flag is the highest bit of the number, so `bits()` returns a negative
number for a security group.
