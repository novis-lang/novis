Any operation combining a `secret` operand with a non-`secret` one produces a `secret` result — the
poisoning shape `rule:security/taint-propagation` defines, applied to this axis **independently**. A
`secret tainted string` interpolated with a plain `string` stays `secret tainted string`; each axis
tracks on its own.

A successful checked `as` conversion **removes `secret`**, mirroring the `tainted` rule for grammar
and implementation consistency rather than because the underlying justification transfers — it does
not. A shape proof says nothing about confidentiality, so a numeric credential that round-trips
through `as uint` becomes a plain, loggable, displayable value with no diagnostic. **That is a known,
accepted gap**, chosen for consistency and recorded here rather than discovered later.

Conversions between `string` and `bytes` preserve `secret` in either direction, the same way they
preserve `tainted`.
