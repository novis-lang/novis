A comparison whose two static types are **disjoint** — no single value inhabits both — is a compile
error (`E0466`), because the compiler already knows the answer and the author did not mean to write
it.

```
string $s = "1";
int $n = 1;
if ($s == $n) { }               // E0466: `string` and `int` are disjoint
if ($s == ($n as string)) { }   // convert once, deliberately, then compare
```

Disjointness, not identity of types, is the test, so ordinary code still compiles: the same type; any
pairing of `int`, `uint`, `float` and `decimal`, which are **one numeric domain**; `?T` against `null`
or against `T`, which is the null test and narrows; a union against any type one member can hold; a
single-value or enum-case type against its base; `mixed` against anything; a class against itself or an
ancestor.

Refused: a non-nullable type against `null`; `string` against `bytes`; an enum against its underlying
integer, where `$e as int` is the written spelling; two unrelated classes; and anything else disjoint.

This turns the entire class of comparisons that silently answered `false` — or answered `true` in PHP
7 and `false` in PHP 8 — into a diagnostic at the site that wrote it.
