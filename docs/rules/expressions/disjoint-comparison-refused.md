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

This turns the entire class of comparisons that could only ever answer one constant into a diagnostic
at the site that wrote it. `$row['id'] == 1` against a string column and `$s == 0` against a
non-numeric string are the common cases.

**Declined:**
- **A statically disjoint comparison answers `false`**, the way a run-time mismatch does
  (`rule:expressions/mixed-equality`). The compiler can prove the answer, and code whose answer is
  known before it runs is a mistake worth naming, the same argument definite assignment makes.
