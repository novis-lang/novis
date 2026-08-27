# Handoff

## State

**M4 — language completeness.** `empty(...)` runs end to end, in value and statement
position, as ADR 0035 § 2's truthy table negated over any expression. The checker half is
`mwl_types::expr::presence::check_empty_operand`
(`crates/mwl-types/src/expr/presence.rs:75`), which shares `isset`'s guarded-subscript
marking (`mark_guarded_subscripts`, `:99`) and none of its E0498 shape check; the lowering
is one line per position calling `Lowering::lower_not`
(`crates/mwl-ir/src/lower/expr.rs:295`, `crates/mwl-ir/src/lower/stmt.rs:311`).
`truthy_convert` gained the `Ty::Null` row it had asserted was unreachable, which also
closed `!null` and `if (null)`.

**The language frontier is now two slices, both with file sets of their own**, and neither
is the one-liner `empty` was. `exit` needs a distinguished unwind; a static property has no
storage anywhere below the checker.

`verify.py` 6 of 6 green — conformance **610**, differential **172**, 1632 unit tests.
`python tools/holes.py` reads **24 sites, 6 items**, unchanged: every remaining site is a
catch-all, and `empty` was never one of them.

## Next group

**The static property first, then `exit`.** They share no files with each other and none
with this session; the static property is picked first because it is a *hole* (the checker
accepts a shape nothing below it lowers) while `exit` is a feature that was never built.
Read `crates/mwl-ir/src/ir.rs:290-320` (the `InstKind` neighbourhood a new pair joins)
before either.

- [ ] **A static property lowers nowhere, read or write** — the read falls through
      `lower_expr`'s dispatch catch-all (`crates/mwl-ir/src/lower/expr.rs:300`) and the
      write through `lower_stmt`'s reassignment arm
      (`crates/mwl-ir/src/lower/stmt.rs:1065`); `ExprKind::StaticPropertyAccess` is
      `crates/mwl-syntax/src/ast.rs:775`. **There is no storage for one anywhere**: `grep`
      finds no static slot in `crates/mwl-ir/src/ir.rs`, no `Class` field holding one and
      no runtime home, so this is an `InstKind` pair plus a codegen data slot plus a
      runtime table, not a lowering arm. **Decide the lifetime before writing any of it**
      and record it in `docs/adr/README.md` § *Decisions taken at project start*: priority
      1 makes state request-scoped, so a process-global static is the wrong default under
      `mwl serve` at M7 even though it is what a single `mwl run` cannot tell apart. No
      worklist item names this, so `holes.py` does not count it.
- [ ] **`exit` and `exit(...)`** — its own file set (`crates/mwl-runtime/src/abi.rs:53`'s
      `Fault`, `mwl-cli`'s exit status). There is no `process::exit` in this tree and there
      must not be one in a helper, for the same priority-1 reason: under `mwl serve` a
      helper ending the process ends every other in-flight request with it. It wants a
      distinguished unwind carried to `mwl-cli`'s exit status.

## Backlog

- An enum case tagged into a `mixed` reads as its backing integer, so a case backed by `0`
  is falsy where ADR 0035 § 4 makes every statically typed one truthy —
  `mwl_codegen::ty::tag_of` reserves no enum tag. `Core\Reflect::typeOf` is the same gap by
  a second route.
- `array<T> as array<U>` does not lower (`crates/mwl-ir/src/lower/expr.rs:877`), which is
  what keeps several `gaps.py --errors` sites unreachable from source — ADR 0007 § 2.
- The value form of `require` (`$c = require 'config.mwl';`, ADR 0021 § 3) lowers nowhere —
  `mwl-ir`'s crate doc owns the gap.
- `Class::method(...)`, the first-class callable spelling, panics `mwl-ir` — gap 1 in that
  crate's own doc.
- 17 of the 32 named `.mwlt` cases `python tools/loop.py --list` schedules are still to
  write; conformance is 610 against the goal's 750.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
