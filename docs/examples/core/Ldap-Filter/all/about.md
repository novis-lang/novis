Builds a filter that finds only the directory entries that every one of the given filters matches.

This is how you combine conditions, such as "is a user account" and "has a mail address". Give it one
filter or more. A filter you combine can itself be made with `all`, `Core\Ldap\Filter::any` or
`Core\Ldap\Filter::not`, so you can build any condition. A filter is a value that does not change, so
you can use the same one in several combinations.

No filter at all throws a `LogicError`.

**Good to know:** filters can be put inside each other up to 100 levels deep. One more level throws a
`LogicError`. A filter that a person writes never comes close to this.
