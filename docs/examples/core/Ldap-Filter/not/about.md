Builds a filter that finds the directory entries that the given filter does not match.

Use it to leave something out, such as the accounts without a mail address, or the computers among all
accounts. LDAP has no "greater than" and no "less than" filter, and `not` is how you write them:
"not at most 5" means "greater than 5".

**Good to know:** an entry that has no value for the attribute is not matched by the inner filter, so
`not` matches it. Filters can be put inside each other up to 100 levels deep, and one more level
throws a `LogicError`.
