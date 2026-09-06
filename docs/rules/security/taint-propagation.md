Any operation combining a tainted operand with an untainted one — concatenation, interpolation, a
string member, an array of scalars — produces a tainted result. This is the poisoned shape the type
system already uses on other axes, applied to a new one, and `secret` poisons independently beside it
(`rule:security/secret-propagation`).

A checked `as` conversion to a type that already throws on a malformed shape — `as uint`, `as int`,
`as float`, `as bool`, an enum's backing type — **removes the qualifier on success**. No new syntax is
needed: a value that survived the check has had its shape proven, which is what laundering means for a
non-string type. `as ?T` decides the qualifier by exactly this rule and launders nothing of its own
(`rule:expressions/conversion-keeps-qualifiers`).

`bytes as string` and `string as bytes` **preserve** the qualifier in either direction. UTF-8 validity
says nothing about whether the content is safe for a given sink, and a conversion that laundered here
would be a one-word bypass of every rule below.
