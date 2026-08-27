# Handoff

## State

**M4 — language completeness.** The `mwl-ir` *expression* dispatch's catch-all now has no
reachable target: `ExprKind`'s 45 variants are covered by 34 arms and the eleven left are
each a diagnostic or dead in the parser. The subtraction is in the arm's own doc comment at
`crates/mwl-ir/src/lower/expr.rs:@lower_expr`, and it is the proof — the panic message is not.

- **`Foo::class` runs** (`mwl_types::expr::members::check_class_name_const`). It folds to the
  class's fully qualified name as a `string` constant, recorded as the same
  `ExprInfo::CoreConst` a `Core` class constant already travels in, because both are "a
  constant with no storage to read it back from" and `mwl-ir` cannot name a `QName`.
  `self::class` and `parent::class` fold with it; an undeclared written name is the ordinary
  `E0303`, which is where MWL parts from PHP (PHP folds `Bogus::class` because the string is
  on its way to `new $name`, and ADR 0011 has no such spelling).
- **`$obj::class` and `static::class` are `E0702`.** The second is the one that would be
  *silently* wrong: ADR 0008 binds `static` at the call site, so folding it answers the
  declaring class. The frame carries a `Ty::ClassDesc`, so a future lowering can answer it.
- **`throw` lowers in expression position** (`lower_expr`'s own arm, which adds the
  unreachable continuation block `lower_throw`'s seal leaves the caller needing). What made
  it usable is one line in `mwl_types::ty::TypeInterner::make_union`: **`never` is absorbed
  by a union with any other member**, so `$v ?? throw new LogicError(…)` satisfies a plain
  `string`. `exit` in the same position benefits identically. Still open: `is_assignable`
  gives `never` no bottom-type row at all, so a bare `string $s = throw …;` is `E0401`.
- **Three refusals**, each named in `mwl-diagnostics`' registry with its reason: `yield` used
  as a value is `E0448` (ADR 0053 § 5 — no `send()`), `spawn script` is `E0703` (M5) and
  `require` used for its *value* is `E0704` (`mwl-ir` gap 22, whose text now says so). The
  statement/value split for `yield` and `require` lives in one place —
  `mwl_types::expr::check_expr_stmt`, called from `locals::check_stmt`'s `StmtKind::Expr`
  arm — and mirrors `lower_expr_stmt`'s, parenthesis handling included.

Dead without work: `ConstFetch` is `E0319`, bare `self`/`static`/`parent` are `E0321`,
`Assign { by_ref: true }` is `E0701`, `Error` dies in the parser, and `yield`'s other three
spellings were already `E0448`.

## Next group

**The `mwl-ir` operator catch-alls** — the same "enumerate the roster, do not trust the
message" job, both in the file this session already had open. The file set:
`crates/mwl-ir/src/lower/expr.rs`, `crates/mwl-syntax/src/ast.rs`, `tests/conformance/lang/`.

- [ ] **`expr.rs:2746` — the unary-operator catch-all** (`holes.py --item 6`'s neighbour).
      `UnaryOp` is a short roster; subtract the three arms above it (`-`, `!`, `~`) and write
      one scratch `.mwl` per survivor before assuming any is dead. Anchors:
      `crates/mwl-ir/src/lower/expr.rs:2746`, `crates/mwl-syntax/src/ast.rs:222` (`UnaryOp`).
- [ ] **`expr.rs:3192` — the binary-operator catch-all.** Same shape against `BinaryOp`, and
      note that `And`/`Or`/`Coalesce`/`Concat` and the object-comparison rows never reach it —
      `lower_expr` splits them off before `lower_binary` is called at all, so the roster to
      subtract from is what is left. Anchors: `crates/mwl-ir/src/lower/expr.rs:3192`,
      `crates/mwl-syntax/src/ast.rs:247` (`BinaryOp`), `crates/mwl-ir/src/lower/expr.rs:197`
      (the `ExprKind::Binary { .. }` arm that routes here).
- [ ] **`expr.rs:384`/`629`/`762` — the three dispatch catch-alls one level down.** Cheap to
      judge once the two above are done, and they share the same rosters.

## Backlog

- `is_assignable` has no bottom-type row for `never`, so `string $s = throw …;` and
  `string $s = exit(1);` are `E0401` — `crates/mwl-types/src/expr/assign.rs`.
- `mwl-ir` gap 22: a `require`d file's own top-level statements do not run; `E0704` is what
  its session deletes — `crates/mwl-ir/src/lib.rs:@known gaps`.
- `static::class` is answerable from the frame's `Ty::ClassDesc` and is not today — the
  `E0702` doc comment in `crates/mwl-diagnostics/src/lib.rs` names the shape.
- A user-declared class constant's value is unmodeled in `mwl_types`, which is the live half
  of `lower_expr`'s `ClassConstAccess` panic — `crates/mwl-types/src/expr`'s known gaps.
- `docs/spec/02-php-migration.md` has no row for `::class`; `check-migration.py` scores that
  file and this session did not touch it.
