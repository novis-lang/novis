`expr as ?T` produces the value that `expr as T` would produce where that succeeds, and `null` where
it would throw. Every other property of the conversion — what counts as success, the whole-string rule
for `string → int`, the range checks — is the checked form's, unchanged. The operator adds no notion
of validity of its own; it only changes what happens to a failure.

```
var $id   = Core\Request::query('id') as ?uint;      // ?uint — null if absent, or not a uint
var $page = Core\Request::query('page') as ?uint ?? 1;
var $mode = Core\Request::query('mode') as ?SortMode ?? SortMode::Newest;
```

A `null` operand that `as T` rejects yields `null` rather than a diagnostic, which is what keeps the
common shape one line. Where `as T` accepts `null`, `as ?T` gives the same value: `null as string` is
`""`, so `null as ?string` is `""` too. The cost is stated plainly: at such a site an **absent** value and a **malformed** one both
reach the `??`. Where that distinction matters, test the operand for `null` before converting, or use
the throwing form — a value that gates access is converted with plain `as T`.

There is one definition and no family that escapes it: every `as ?T` is the `as T` of the conversion
table, with `null` where it throws (`rule:expressions/nullable-conversion-availability`).

The form exists because an absent page number or an unfilled filter is an ordinary outcome, not an
error, and the only other way to reach it is to check and then convert. That parses the same bytes
twice on the request path and needs two definitions of success to agree forever; one operator over
one table is one parse and one definition. `?T` is already the one way to write a value that may be
absent (`rule:classes/no-undefined-value`), so the operator produces an existing type and adds no
syntax: `as` already takes a type, and `?int` is one.

**Declined:**
- **Refusing a nullable operand.** It keeps "missing" and "invalid" apart in the syntax, but it does
  not prevent the conflation: `($raw ?? '') as ?uint ?? 1` collapses the same two states. It would
  only charge a line on the most common shape in the language.
- **A `Core` member per scalar target, such as `Str::toIntOrNull` or `Int::tryParse`.** The name
  shapes are banned (`rule:core-api/verb-lexicon`), and it needs one member per target type forever,
  where an operator covers every type, including ones added later.
- **A `Result` or `Either` return that says why a conversion failed.** There is no sum type, and
  adding one to avoid a `null` test is a decision about the whole language.
- **Rewriting PHP's `(int)$x` to `$x as ?int ?? 0`.** It would restore the silent zero that
  `rule:types/no-legacy-cast` removes, and it is not even faithful: `(int)"12abc"` is `12` in PHP and
  `null` here. A converter names both forms and lets the author choose.
