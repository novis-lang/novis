Builds a filter that finds the directory entries that at least one of the given filters matches.

Use it when there is more than one right answer, such as a login that may be an account name or a mail
address. Give it one filter or more. Combine it with `Core\Ldap\Filter::all` to say which conditions
must always hold.

No filter at all throws a `LogicError`.

**Good to know:** filters can be put inside each other up to 100 levels deep. One more level throws a
`LogicError`. A filter that a person writes never comes close to this.
