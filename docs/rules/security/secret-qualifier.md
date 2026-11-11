`secret` is a second, independent compile-time qualifier on three carriers: `string`, `bytes` and
`mixed`. It and `tainted` are
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

**A `secret` value is never an array key.** An array's type records nothing about its keys, so
`Core\Arr::keys`, a `foreach`, a dump and an error message all read a key back as plain text. No
element type can change that, so a key whose type contains `secret` is `E0896` in every array,
`array<secret string>` and `array<mixed>` included. It is asked at every position a value becomes a
key, the same positions `rule:security/taint-propagation` lists for a `tainted` key: a subscript write,
every compound write, and a literal's key in every position, an argument's included. The cost is that
a program keys by something that is not secret, such as an id, and keeps the secret in the element.

**A `secret` value enters `mixed` only where `secret mixed` is written.** `mixed $m = $secret;` does
not compile, and its help names `secret mixed`. The refusal is in the assignment relation, so it holds
at every placement: a declaration, an assignment, a property, a return, a parameter, a `foreach`
variable, and `array<secret string>` into `array<mixed>`. `secret mixed` takes any value, and a value
read out of it is `secret` too: an element read is `secret mixed`, and text out of it through `as` or
`is` is `secret tainted string`, since text out of any `mixed` is `tainted`. Every sink refuses it. A
`Core` parameter declared `mixed` still takes a `secret` argument, because each `Core` sink that
declares one refuses the value at the call with its own code
(`rule:security/secret-sinks-refuse`). There is no `tainted mixed`: `mixed` already carries `tainted`.
`secret mixed` is `mixed` below the checker, so it costs nothing at run time.

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
