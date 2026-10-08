`Core\Taint::assertTrusted(tainted string, string $reason): string` is the one generic escape from the
qualifier, for the case where the developer has validated the value themselves and needs to say so. It
is modelled on this project's own `unsafe` policy: forbidden by default, rare, greppable, and carrying
a written reason at the call site rather than a silent cast. Its `bytes` form is a second member,
`Core\Taint::assertTrustedBytes(tainted bytes, string $reason): bytes`, under the same rules, as
`Core\Secret::revealBytes` is `reveal`'s; its case is a program decoding with `Core\Serialize::decode`
the bytes it encoded itself (`rule:classes/serialize-is-a-closed-format`).

It is the answer at every position that has no launderer *and cannot have one* — a metric label, whose
hazard is unbounded cardinality rather than content (`rule:security/metric-label-refuses-tainted`); a
regex pattern, where no transform makes an attacker-authored pattern safe
(`rule:security/regex-pattern-is-a-sink`); a format template drawn from a translation catalogue
(`rule:security/every-grammar-is-a-sink`).

It removes `tainted` and nothing else: a `secret` operand is refused there, because confidentiality is
a separate axis and this member makes no claim about it.
