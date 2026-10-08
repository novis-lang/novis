Any operation combining a tainted operand with an untainted one — concatenation, interpolation, a
string member, an array of scalars — produces a tainted result. This is the poisoned shape the type
system already uses on other axes, applied to a new one, and `secret` poisons independently beside it
(`rule:security/secret-propagation`).

**Text out of `mixed` is `tainted`.** `mixed` is where request input lands and the qualifier cannot be
written on it (`rule:security/tainted-qualifier`), so every way text leaves it adds the bit: `as string`
and `as ?string` answer `tainted string` and `?tainted string`, `as bytes` answers `tainted bytes`,
`as array<string>` answers `array<tainted string>`, `as secret string` answers `secret tainted string`,
and a `.` or an interpolation with a `mixed` operand is tainted. `$m is string` narrows `$m` to
`tainted string` on the true edge, and over a `mixed` subject `is tainted {…}` and `is tainted string`
are admitted. The same holds for an `array<mixed>` operand and for a union with a `mixed` member.
**The keys of such a value are outside text too**: a `foreach` key over a `mixed` or `iterable`
subject, or over an array whose element type carries `tainted` or `mixed` anywhere — `queryArray`,
`postArray`, `headers()`, a decoded `array<mixed>` — is `tainted string`, because an array's type says
nothing of its keys and its elements are the evidence of where it was filled. An object's fields are
read the same way: a shape target of `as` or `is` over a class takes each text field from the field of
that name, so a field that is `mixed`, `object` or tainted, or one the class does not declare and only
a subclass could, counts as outside text. Where the text would land in a type the program *wrote* — a
shape target of `as` or `is` over a `mixed` or `object` operand or over such an object, or a `foreach`
key or value binding — the qualifier is never added behind the declaration: a field or binding written
without `tainted` is a diagnostic asking for `tainted {…}` or `tainted string`, and a `var` key takes
`tainted string` itself. A literal that was stored in `mixed` and is genuinely trusted is recovered
with `rule:security/assert-trusted`, or with the sink's own launderer.

A checked `as` conversion to a type that already throws on a malformed shape — `as uint`, `as int`,
`as float`, `as bool`, an enum's backing type, a set of allowed values — **removes the qualifier on
success**, from a tainted operand and from a `mixed` one alike. No new syntax is needed: a value that
survived the check has had its shape proven, which is what laundering means for a non-string type.
`as ?T` decides the qualifier by exactly this rule and launders nothing of its own
(`rule:expressions/conversion-keeps-qualifiers`).

`bytes as string` and `string as bytes` **preserve** the qualifier in either direction, and so does a
conversion out of a `?tainted string` or an `array<tainted string>`. UTF-8 validity says nothing about
whether the content is safe for a given sink, and a conversion that laundered here would be a one-word
bypass of every rule below. All of it is decided while checking: no tag at run time, nothing on the
request path.
