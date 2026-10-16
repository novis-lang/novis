```
duration := ( DEC_INT unit )+
unit     := ns | us | ms | s | m | h | d | w
```

- **One token.** `1h30m` lexes as a single duration token, maximal munch, not as three tokens.
- **Units strictly descend and may not repeat.** `1h30m` is accepted; `30m1h` and `1h1h` are lexer
  errors naming this rule, so there is exactly one spelling of any given constant.
- **Only after a plain decimal integer** — never after `0x…`, `0b…`, a float or an exponent, so `0x1d`
  stays a hex literal and `1.5s` is an error rather than a rounded duration.
- **Lower case only**; `30S` is a diagnostic, not a second spelling.
- **No sign.** `-7d` does not parse; a backwards step is `->minus(7d)`.
- `d` is exactly 24 h and `w` exactly 168 h. A *calendar* day is a `DateTime` unit and never a
  `Duration` at all.

The type is `Core\Time\Duration`, always — the suffix *is* the type, with nothing
untyped-until-placed about it (`rule:types/numeric-literal-placement`). A duration written this way is
folded at compile time to a single nanosecond count, so the grammar is never read at run time, and a
duration beyond `Duration`'s range is a compile error, not a wrap. At run time it is one
`Duration::nanoseconds` call on that count.

`1h + 30m` does not compile — there is no operator overloading; write `1h30m` or `$a->plus($b)`. So
does `1h30m as int`; write `->toSeconds()`. `$n s` is not a duration; a computed count is
`Duration::seconds($n)`.

**One grammar, three places, one parser**: source, `Duration::parse($s)` at run time, and a `"30s"` in
`nvs.toml` at boot. `Duration`'s string form emits this grammar too, so a value round-trips through
`parse` over exactly the durations the grammar can spell — the non-negative ones. A negative duration
renders `-1h30m` for a reader, and `parse` refuses that leading `-` **by name** rather than reading a
positive value out of it. Sharing one parser means a grammar change cannot land in one place and miss
the others, the property `rule:expressions/intrinsic-constant-arguments` requires of every intrinsic.

The literal exists because `rule:core-api/units-are-types` sends its cost to the call site:
`Duration::seconds(30)` at every timeout, TTL, sleep and retry is the pressure under which untyped
`int $seconds` parameters grow back. `rule:core-api/no-methods-on-scalars-or-arrays` rules out `30.seconds`,
so a literal is the only short form left. The grammar is Go's `ParseDuration` with `d` and `w` added,
which is already what Kubernetes, Docker, Prometheus and Caddy configuration accept. `d` and `w` are
kept because retention and cutoff windows are the common case, and the calendar ambiguity Go avoids by
stopping at `h` is closed by the calendar day being a `DateTime` unit. The missing sign means the parser
never has to decide whether the `-` in `$a -7d` is binary or part of a duration.

`nvs fmt` (`rule:tooling/fmt-is-one-canonical-style`) never rewrites a duration: `90m` and `1h30m` are
the same value, and choosing between them is the author's, as `0x10` against `16` is.

**Memory:** each evaluation of a written duration builds one `Duration` object from the folded count,
the same as a computed `Duration::nanoseconds($n)`.

**Declined:**
- **`Duration::seconds(30)` alone, with no literal.** The verbosity is paid in every program, and it is
  the pressure that brings untyped integer parameters back.
- **ISO-8601 durations, `PT30S` and `P7D`.** `PT1H30M` reads far worse inline than `1h30m`. An ISO
  reader for wire formats is a different job from a literal.
- **`3.days`, as Kotlin and Scala write it.** A method on a scalar is what
  `rule:core-api/no-methods-on-scalars-or-arrays` refuses, and an exception for one type is how that
  rule's twin problem grows back.
- **A general units-of-measure system, such as F#'s `9.81<m/s^2>`.** It is a type-system feature of its
  own, with unit algebra in signatures and inference through arithmetic, for a domain where time is the
  only unit that recurs (`rule:programs/memory-priority`, simplicity).
- **`1h + 30m` through operator overloading on `Duration`.** Novis has no operator overloading and does
  not add it for one type; `1h30m` already covers the case.
- **A sign in the grammar.** It would put a form into `nvs.toml` and the lexer that source refuses as
  `-7d`, and the three places would stop being one grammar.
- **A relative-time string such as `"+2 weeks"` on `DateTime`.** It duplicates `startOf`, `plus` and
  `next`, it is a mode string, and it hides whether a calendar or an exact offset was meant.
