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
a compile-time constant folded to a single nanosecond count and emitted into the constant pool, so
`{timeout: 30s}` allocates nothing at run time and a duration beyond `Duration`'s range is a compile
error, not a wrap.

`1h + 30m` does not compile — there is no operator overloading; write `1h30m` or `$a->plus($b)`. So
does `1h30m as int`; write `->toSeconds()`. `$n s` is not a duration; a computed count is
`Duration::seconds($n)`.

**One grammar, three places, one parser**: source, `Duration::parse($s)` at run time, and a `"30s"` in
`nvs.toml` at boot. `Duration`'s string form emits this grammar too, so a value round-trips through
`parse` over exactly the durations the grammar can spell — the non-negative ones. A negative duration
renders `-1h30m` for a reader, and `parse` refuses that leading `-` **by name** rather than reading a
positive value out of it.
