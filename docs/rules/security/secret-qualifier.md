`secret` is a second, independent compile-time qualifier on `string` and `bytes`. It and `tainted` are
independent bits rather than a combined enum: a value can be plain, `tainted`, `secret`, or `secret
tainted`. When both are spelled together, **`secret` comes first** — the only accepted order, and
`tainted secret string` is a diagnostic naming the required order rather than a second valid spelling
of the same type.

Like `tainted` it is a reserved keyword and a production in the type grammar, checked once and erased
before codegen, so a `secret` value costs nothing at run time
(`rule:security/tainted-qualifier`).

The qualifier is a property of a **type**, so a container carries it only where the container's own
element or field type spells it. `array<secret string>` and `{token: secret string}` keep a secret,
and a value read back out of either is still `secret`; writing one into an element or a field whose
declared type does not carry the qualifier is refused where it is written, because that write is the
last place the bit is visible. A written argument is the one position not asked — three of them take
a credential legitimately (`rule:security/secret-sinks-refuse`), and what an argument owes is that
rule.

Two orthogonal questions — trust and confidentiality — get their own checked axis instead of being
conflated into one. `tainted` asks whether a value's shape is safe for a sink; `secret` asks where the
value may go at all. A well-shaped API key is still catastrophic to leak, and a username safe to
display is still attacker-controlled. They compose for the case that matters most in practice, a
submitted password, without needing a third combined concept.

- **Memory:** none. The qualifier is erased before codegen, so a `secret` value is the same bytes as
  the plain one.

**Declined:**
- **An opaque `Core\Secret` wrapper type instead of a qualifier.** It would be a second mechanism
  beside `tainted`'s, with a wrapper's usual friction: no interpolation, and an explicit unwrap at
  every use.
- **Zeroing a `secret` value's memory when it goes out of scope.** That needs a destructor-shaped hook,
  and there are none (`rule:classes/no-destructors`); the qualifier stays compile-time only.
