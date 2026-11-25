When the subject of a `match` has the static type of an enum `E`, or `?E`, and the `match` has no
`default` arm, every case of `E` must be named by an arm, and `null` too for `?E`. A missing case is
**`E1016`** at the `match` keyword, and the message lists every missing case. The check is
`crates/nvs-types/src/expr/cover.rs`.

An arm condition covers a case when it is a case of `E`, or a class constant that folds to one
(`rule:classes/a-constant-value-is-folded-when-the-program-compiles`). Any other condition covers
nothing, so that `match` needs a `default`. A subject of `mixed`, of a union with anything but
`null`, or of `bool`, as `match (true)` is, is not checked, and a value no arm takes there throws
`LogicError` at run time. Missing cases are listed by value, and a `Core` case is named the way a
program writes it after a `use`, such as `ExitReport::Reason::Finish`.

A statically typed `E` is always one of its cases, because every write is checked
(`rule:enums/representation`). So a `match` that names every case lowers no fallback throw, and does
not compare its last label either: the last arm is the only value left, and the lowering jumps to it.
The checker's verdict reaches the lowering as `ExprInfo::MatchCoversEveryCase`, recorded at the
`match` keyword's span. That is one branch less per evaluation, counted by an IR test
(`rule:programs/less-work-is-taken`).

The editor's quick fix for `E1016` is the error's own suggestion: one arm per missing case, each
throwing `LogicError` that names its case, so the file compiles and the author replaces each body.
It never inserts a `default` arm, because that would switch the check off.

**Cost.** Check time only, and one branch less per `match` over every case. Nothing at run time and
no memory. A `match` that names fewer cases than its enum has is a compile error instead of a throw
on the request path.

**Declined:**
- **A `switch` over an enum.** It is a statement, and a case it does not name runs nothing, which is
  a valid program.
- **A warning on a `default` arm when every case is named.** A `default` that catches a future case
  is a choice the author makes.
- **A warning instead of an error.** A closed type is checked, and a missing case is the same kind of
  mistake as a missing return.
