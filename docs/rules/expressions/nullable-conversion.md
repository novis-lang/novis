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
