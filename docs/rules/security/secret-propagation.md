Any operation combining a `secret` operand with a non-`secret` one produces a `secret` result — the
poisoning shape `rule:security/taint-propagation` defines, applied to this axis **independently**. A
`secret tainted string` interpolated with a plain `string` stays `secret tainted string`; each axis
tracks on its own. The operand is read as far as `tainted` is read: a `?secret string`, a union with a
`secret` member and an `array<secret string>` poison `.`, `.=` and interpolation exactly as a
`secret string` does, and an `as` conversion out of one keeps the bit on the target —
`?secret string as string` is `secret string`, `array<secret string> as array<string>` is
`array<secret string>`.

**A `secret` value converts only to a type that can carry `secret`**: `string`, `bytes`, and a
nullable, an array or a union with one of them. `$secret as int`, `as uint`, `as float`, `as bool`, an
enum, a class and `mixed` do not compile (`E0894`), and the help names
`Core\Secret::reveal($value, "reason") as int`. A shape conversion over a class whose shape names a
`secret` property is the same refusal, since a shape field cannot be `secret`. A shape proof says
nothing about confidentiality, so the checked conversion that launders `tainted`
(`rule:security/launderers-are-sink-named`) does not launder `secret`. A numeric credential pays one
conspicuous call, and the reason it gives is read where the value stops being secret.

Conversions between `string` and `bytes` preserve `secret` in either direction, the same way they
preserve `tainted`.

- **Memory and latency:** none. The check runs in the compiler, and the qualifier is erased before
  codegen.

**Declined:**
- **A successful checked conversion removes `secret`, as it removes `tainted`.** It kept the two axes
  alike, and it let a numeric credential that round-trips through `as uint` become a plain, loggable
  value with no diagnostic.
- **A `secret` form of every scalar type.** It adds a qualified `int`, `uint`, `float`, `decimal` and
  `bool`, a rule for every operator, and a class for every numeric `Core` parameter. That is language
  surface no program has needed, and `Core\Secret::reveal` covers the numeric credential.
