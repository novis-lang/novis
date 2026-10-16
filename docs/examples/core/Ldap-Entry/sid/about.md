Returns the one value of an attribute as a `Core\Ldap\Sid`, such as an entry's `objectSid`.

A SID (security identifier) is the id Windows uses for a user, a group or a computer. The directory
sends it as bytes. `sid` reads those bytes and returns a `Core\Ldap\Sid`. With it you can print
the SID as text, such as `S-1-5-21-…-1107`, read its last number with `rid`, or find its domain
with `domain`.

If the entry has no value for the attribute, the result is `null`. For an attribute with several
values, `sid` throws a `LogicError`, and `sids` returns every value. `sid` also throws a
`LogicError` when the value is not a SID, and the message names the attribute.
