A member never takes a mode as a string or as an integer constant. There is no `fopen($p, "r+b")`, no
`hash("sha256", …)`, no `MB_CASE_TITLE`. A mode is an enum (`rule:enums/closed-integer-type`), always, and
where a member accepts only some of an enum's cases it declares that closed subset in the signature
(`rule:types/single-value-types`) so an unsafe case is a compile error naming the reason.

A **grammar** is not a mode string and is not covered. A regex pattern, a `printf` template, a CLDR date
pattern and a `pack` format each express something no enum can, and all four are compile-time-checked
intrinsics (`rule:expressions/intrinsic-constant-arguments`). There are exactly four, and the list is closed.

The gain is that every mode is typo-proof, completable in an editor and exhaustively matchable. The cost is
one enum declaration per mode family, which is also what makes them documentable one case at a time
(`rule:core-api/reference-card`).
