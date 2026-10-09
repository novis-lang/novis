Returns a filter that matches the entries where a number attribute has every one of the given bits
set. Active Directory keeps many settings as bits in one number, such as `userAccountControl` and
`groupType`.

Use it to find, for example, the accounts whose password never expires (bit `65536`).

**Good to know:** give several bits as one number, such as `2 | 512`. The attribute name cannot be
`tainted`. A name that is not an attribute name throws a `LogicError`.
