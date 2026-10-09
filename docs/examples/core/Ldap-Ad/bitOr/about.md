Returns a filter that matches the entries where a number attribute has at least one of the given
bits set. Active Directory keeps many settings as bits in one number, such as `userAccountControl`
and `groupType`.

Use it to find, for example, the accounts that are disabled or locked out.

**Good to know:** give several bits as one number, such as `2 | 16`. The attribute name cannot be
`tainted`. A name that is not an attribute name throws a `LogicError`.
