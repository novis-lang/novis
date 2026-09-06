`$a == $b` where either side is `mixed` or a union compiles, dispatches on the runtime tags, and
applies `rule:expressions/equality-semantics`'s table to the row they land in.

**When the two runtime tags belong to different rows, the answer is `false`.** It does not throw, and
it does not convert.

That is the strict answer, not a looser rule returning: a `string` and an `int` are not the same
value. The static case is a compile error rather than `false` for a different reason — there the
compiler can prove the answer before the program runs, which makes it dead code rather than a
question (`rule:expressions/disjoint-comparison-refused`).

The asymmetry is worth stating plainly: **`mixed` is where you pay for not declaring a type.** A
comparison against `mixed` silently answers `false` where a typed one would have refused to compile.
