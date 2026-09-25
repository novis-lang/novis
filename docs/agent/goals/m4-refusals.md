---
milestone: M4
---
# Loop goal 54 — every shape the checker admits lowers, or a diagnostic naming its rule refuses it

A program that type-checks compiles. None of the sixteen places where `nvs-ir` type-checks a shape and
then panics is left standing: each shape a program can write either lowers, answering as PHP does, or
is refused where it is written by an `nvs_types` diagnostic that names its rule, with a conformance case
that expects it. `CEILING` in `crates/nvs-ir/tests/refusals.rs` is `0`, carried-refusals.md's entry for
M4 is gone, and M4's promise that "a program that type-checks is a program that runs" holds.

## Why here

The first of the closure goals, after goal `gap-register`, because that goal's roster is what checks
"nothing tagged to M4 is left" and every closure goal ends on that claim. M4 goes first because it is
the oldest milestone with a promise still open, and because the milestones after it compile through
this lowering: a refusal left in `crates/nvs-ir/src/lower/` is a program that M5 through M8's surfaces
cannot express either.

What it needs is already built:
- the checker's diagnostics for every invariant site: `E0234`, `E0439`, `E0443`, `E0497`, `E0700` and
  `E0723`, in `crates/nvs-diagnostics/src/lib.rs`;
- `rule:expressions/one-equality-operator`'s cross-representation rows, which the label sites reuse;
- the tag-dispatching helper family (`Helper::ValueTruthy`, `Helper::Identical`, the erased member
  access), which the tagged operands reuse.

What needs this goal is goal `m5-proofs` and every closure goal after it. Each writes cases through
this lowering, and each would otherwise meet a panic here that is not its own.

## Stage 0 — the catch-up

These sentences on disk become wrong when this goal lands. Each is corrected in the stage that makes it
wrong, and its code comment is rewritten whole. Re-grep before editing: these are anchors, and files
move.

- `crates/nvs-ir/tests/refusals.rs:64-78` — `CEILING`'s doc says goal `type-test`'s open items own the
  `is` site and that the number "comes back to 15". That goal has retired with the site still open.
  Stage 7 rewrites this doc; stage 9 rewrites it again at `0`.
- `docs/agent/carried-refusals.md:25-65` — entry 901 says "fifteen now" and lists anchors that have
  drifted by up to 700 lines. It is not edited. Stage 9 deletes it through its own `[until:]` trailer.
- `crates/nvs-ir/src/lib.rs:187` — "Each panics naming itself rather than miscompiling." Stage 2 adds
  the second spelling of a panic, so this preamble is rewritten there to say which spelling means what.
- `crates/nvs-ir/src/lib.rs:194-207` — gap 1's `switch`/`match` half. Stage 6 closes it, so the gap
  keeps only its `for`-condition half.
- `crates/nvs-ir/src/lib.rs:333-336` — gap 6's sentence on `write_back_array`'s catch-all. Stage 3
  closes that catch-all.
- `crates/nvs-ir/src/lower/expr.rs:5389-5396` — `test_shape`'s own known gap, which calls
  `is array<Foo>` "a decision". Stage 7 takes that decision.
- The `# Panics` paragraphs above each site (for example `call.rs:775-780`, `convert.rs:498-504`,
  `exception.rs:21-25`, `stmt.rs:1470-1476`) — each is rewritten in the slice that closes its site.
- `docs/agent/playbook.md:1430-1436` — this bullet carries the same trailer as entry 901 and retires
  with it in stage 9.

## Stage 1 — the floor

Goal `gap-register`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: a guarded site says which diagnostic guards it

`crates/nvs-ir/src/lower/mod.rs`, `tools/holes.py` and `crates/nvs-ir/tests/refusals.rs`.

Of the sixteen sites, seven cannot be reached from a program. The checker refuses the shape where it
is written, and the site's message even names the code. They stay counted anyway, because the source
has no way to say which kind of site it is. `tools/holes.py:66-77` and `playbook.md:485-491` both name
the fix: make the *source* declare its kind, as `CodegenError` does with `Internal` versus
`Unsupported`.

- **One macro in `crate::lower`, `guarded_by!(code::E_…, "…")`.** It panics with the code at the head
  of its message, so a crash report names the diagnostic that should have fired. Its first argument is
  the `nvs_diagnostics::code` constant itself, so a code that does not exist does not compile.
- **`python tools/holes.py --guarded`** lists every `guarded_by!` site by file, then line, as `--sites`
  does. Each line carries its code and the first `.nvst` under `tests/conformance/` whose expected
  output holds `error[<code>]`, or `NO CASE`. `--sites` does not count these sites, because their
  construct is not `CONSTRUCT`'s. Nothing about `REFUSAL` changes.
- **`every_guarded_site_names_a_code_a_conformance_case_expects`**, beside `refusals.rs`'s existing
  test and in its shape: it shells out to `--guarded` and fails on any `NO CASE` line, naming the site.
  This test is what makes a guarded site *proven* unreachable rather than merely *claimed*
  unreachable. Without it, the macro would be a way to reword a panic past the recognizer, which is the
  move `refusals.rs` forbids by name.

## Stage 3 — the guarded sites

One file set: the seven sites' own lines, plus one reject case where a code has none.

| Site | Guarded by | Proof to find or write |
|---|---|---|
| `crates/nvs-ir/src/lower/call.rs:1170` — a by-reference argument from something other than a place | `E0439`, `nvs_types::expr::args::check_inout_arg` (`crates/nvs-types/src/expr/args.rs:1168`) | a reject case expecting `E0439` |
| `crates/nvs-ir/src/lower/control.rs:974` — a `foreach` key binding outside `string` | `E0723` | a reject case expecting `E0723` |
| `crates/nvs-ir/src/lower/control.rs:985` — a `foreach` subject that is not an `array<T>` | `E0443`, `nvs_types::expr::iteration::report_not_iterable` | see below |
| `crates/nvs-ir/src/lower/expr.rs:5229` — `instanceof` on a subject that holds no object | `E0497` | a reject case expecting `E0497` |
| `crates/nvs-ir/src/lower/mod.rs:2615` — an element write through a root that is no place | `E0700`, `check_write_target` | a reject case expecting `E0700` |
| `crates/nvs-ir/src/lower/stmt.rs:263` — the statement roster's catch-all | its own comment's roster (`:240-262`) | each arm names its own code |
| `crates/nvs-ir/src/lower/stmt.rs:1483` — `unset` on anything but a subscripted element | `E0234`, `check_unset_target` | the `unset` reject cases under `tests/conformance/error/` |

- **Probe the shape before converting a site.** Write each shape as a scratch file under `.agent-tmp/`
  and run it (`playbook.md:2011-2016`). If the shape compiles and reaches the panic, the site is not
  guarded. It moves to the stage its shape belongs to, and the handoff says so.
- **The `foreach` subject is the one to probe hardest.** An un-narrowed `?array<T>` or a `mixed`
  subject erases to `Ty::Tagged`, and `rule:iteration/foreach-subjects` says anything outside its three
  subjects is refused at the subject. If the checker lets one through, the fix is to report `E0443`
  there, never to add a runtime branch here. The reject case for it is named in the goal's checks.
- **`stmt.rs:263` stops being a catch-all.** The match spells every `StmtKind` a body can hold. Each
  shape the comment lists as refused upstream gets its own `guarded_by!` arm naming its own code:
  `E0101`, `E0215`/`E0216`, `E0233`, and the parser's codes for `global`, `goto`, a function-scope
  `static` and a parse error. A new `StmtKind` then fails to compile here instead of panicking at run
  time.

## Stage 4 — `$f(...)` through a callable

`crates/nvs-ir/src/lower/call.rs:789` (`lower_closure_call`). The checker already admits this shape: it
answers `callable` (`crates/nvs-types/src/expr/mod.rs:454-456`).

- `rule:types/callable-is-a-closure` says the only value a `callable` holds is a closure. So `$f(...)`
  lowers to that closure itself, retained once as a fresh owner. No call is made.
- PHP answers a first-class callable of a `Closure` with the same object. A differential case pins
  `$f(...) === $f`. If PHP answers differently, PHP's answer is the rule: lower a fresh closure that
  forwards to `$f`, and the conformance case follows the differential one.

## Stage 5 — `throw` and `clone` through a tag

`crates/nvs-ir/src/lower/exception.rs:28` and `crates/nvs-ir/src/lower/expr.rs:5289`, plus the helper
that throws for a non-object in `nvs-runtime`.

The checker lets a `?Throwable`, `object` or `mixed` operand through, on purpose
(`crates/nvs-types/src/expr/members.rs:447-461`), and `clone` takes anything that can hold an object.
All of these erase to `Ty::Tagged`, and both sites then assert `Ty::Object`. The fix is to lower them.

- **A tagged operand is untagged behind one check.** An object — for `throw`, one inside the
  `Throwable` tree — takes the path a `Ty::Object` operand already takes.
- **Anything else throws PHP's `Error`, in PHP's wording.** `throw null` is "Can only throw objects".
  A non-`Throwable` object is "Cannot throw objects that do not implement Throwable". `clone null` is
  "__clone method called on non-object".
- **One implementation.** This is the same move as the erased member access's non-object receiver
  (`crates/nvs-ir/src/lib.rs:319-324`). It reuses that throw path, or sits beside it in the same helper
  family, and is never a second convention.
- A differential case per operator pins the wording. The case suffix `matches-phps` means PHP is the
  authority.

## Stage 6 — labels and the condition

`crates/nvs-ir/src/lower/control.rs:741`, `crates/nvs-ir/src/lower/expr.rs:1927`, and
`crates/nvs-ir/src/lower/convert.rs:608`.

- **A `switch` label or `match` arm at a representation other than the subject's lowers through the
  comparison `==` lowers that pair with.** `rule:expressions/switch-match-equality` says there is one
  comparison in the language, so there is one here. The label arm calls into the equality lowering,
  which lib.rs gap 19 says covers every row of `rule:expressions/one-equality-operator`. It never
  writes a second table.
- **A pair with no equality row is guarded, not lowered.** The checker refuses it (`E0466`,
  `rule:expressions/disjoint-comparison-refused`), so it becomes a `guarded_by!` naming that code.
- **The truthy conversion gets an exhaustive match.** `Ty::Void` becomes a `guarded_by!` naming the code
  that keeps `void` out of value position (`rule:types/declaration`). `Ty::Ref` is an engine invariant,
  because a cell is only ever read through `InstKind::RefLoad`. It keeps the crate's ordinary panic
  form, and its doc comment names the instruction that guarantees it.

## Stage 7 — `is` answers for every type

`crates/nvs-ir/src/lower/expr.rs:5004` and `test_shape` (`:5376-5396`).

`rule:types/type-test` says `is` is total — "Nothing else is refused." — and its table lists every row.
Each lowers:

- **a shape** — the field walk `as` performs for the same type, called in answering mode, so there is
  one walk and not two;
- **a union** — the members' tests, short-circuiting on the first `true`;
- **an intersection** — the members' tests, short-circuiting on the first `false`;
- **`iterable`** — an `array`, or an object whose class implements `Iterable` or `Iterator`
  (`rule:iteration/two-interfaces`);
- **`callable`** — a closure (`rule:types/callable-is-a-closure`);
- **an `array<T>` whose element type no tag decides** — `array<Foo>`, an array of shapes, an array of
  unions. This is the element walk given a per-element test instead of a tag word.

`test_shape` then answers `Some` for every row, and its `None` and its known gap are deleted. There is
no new diagnostic. `E0711` refusing `as array<Foo>` is a fact about `as`, and `rule:types/type-test` is
explicit that `is` does not inherit it.

## Stage 8 — declared types

`crates/nvs-ir/src/lower/mod.rs:3069` (`lower_decl_type`'s `other` arm) and `:3288`
(`lower_checked_ty`), which holds `erase_checked_ty`'s `_ => return None` at `:3473`.

- **`lower_decl_type` has no catch-all.** It spells every `TypeKind` atom `rule:types/grammar` has. An
  atom the AST can answer is answered with the same erasure `erase_checked_ty` gives its checked type.
- **`erase_checked_ty` has no `_` arm.** The two rows it drops are `CheckedTy::CoreShape` and
  `CheckedTy::TypeVar`.
  - Probe each from source first.
  - If either reaches this function from a program, it gets its erasure. A type variable erases to
    `Ty::Tagged`, the representation of every position that admits more than one runtime shape.
  - If neither can reach it — a `Core` options bag is flattened into per-slot constants, and the checker
    substitutes every type variable — then the arm is an engine invariant. Its doc comment names the
    function that guarantees it, and a unit test in `crates/nvs-ir/src/lower/tests.rs` lowers the
    nearest spelling without reaching it.
- **`lower_checked_ty` then cannot fail, and its `unwrap_or_else` panic goes.**

## Stage 9 — the gate

`crates/nvs-ir/tests/refusals.rs`, `crates/nvs-ir/src/lib.rs` § *Known gaps*, and
`docs/agent/carried-refusals.md`.

- **`CEILING` falls in every slice that closes a site, never in a batch at the end.** It is `0` here,
  and its doc comment is rewritten whole to say what `0` means: a new refusal site is a red test, full
  stop.
- **Every "see the crate docs' known gaps" pointer a closed site carried is gone with the site.** Gaps
  1 and 6 keep only their other halves. No other gap in lib.rs is touched.
- **Entry 901 and `playbook.md:1430-1436` retire through their shared trailer**
  `[until: exists crates/nvs-ir/tests/refusals.rs:const CEILING: usize = 0;]`, via `python
  tools/playbook.py --retire` or the wrap. Neither is edited by hand.

## Standing decisions

- **Closing a site means one of two things and nothing else.**
  - **It lowers**, answering as PHP does, with a case that runs it.
  - **Or it is a `guarded_by!` naming the diagnostic** that refuses the shape where it is written, with a
    conformance case that expects that code.

  Rewording a message so `tools/holes.py`'s `REFUSAL` stops matching is not a close, and neither is the
  ordinary engine-invariant form. The only exception is a `Ty` that no source spelling produces
  (`Ty::Ref`, and whichever of stage 8's two rows the probe proves unreachable). For those, the doc
  comment names the function that guarantees it, and the commit says why.
- **`ALLOWLIST` in `refusals.rs` stays empty.** No shape is kept as a refusal on purpose.
- **A shape a rule says lowers is lowered, never refused.** `rule:types/type-test`'s "Nothing else is
  refused" settles `is`. `rule:types/callable-is-a-closure` settles `$f(...)`. The checker's deliberate
  pass of `mixed`/`object` to `throw` (`members.rs:451-455`) settles the tagged operand. Adding a
  diagnostic to any of these is a language change this goal does not make.
- **A shape a rule says is refused is refused by the checker, never at run time.** Stage 3's `foreach`
  subject is the case in point.
- **PHP is the authority on every answer a lowered shape gives.** A `matches-phps` differential case
  decides the wording and identity questions: the `Error` messages of stage 5, and `$f(...) === $f`.
  The fallback stated in each stage applies if PHP disagrees with what is written here.
- **One implementation per question.** A label compares through the equality lowering, a tagged `throw`
  or `clone` goes through the erased receiver's throw path or its helper family, and the `is` shape
  walk is the `as` walk. A second table for any of them is the copy that eventually disagrees.
- **What it spends** (`rule:programs/memory-priority`): nothing per request, per task or per process.
  `is` over a shape or an `array<T>` is an O(n) walk that allocates nothing. A tagged `throw` or
  `clone` costs one tag comparison before the path it already had. `$f(...)` costs one retain. The
  macro and the new `holes.py` mode are compile-time and tooling only.
- **ADR slots: none.** Every design here is already a rule. `guarded_by!` is an implementation spelling
  in the precedent `CodegenError` set. Its home is `crates/nvs-ir/src/lib.rs`'s § *Known gaps* preamble
  and `tools/holes.py`'s `CONSTRUCT` comment. If a stage genuinely needs a new design, the most it may
  add is one new record and no other number.
- **M4's other acceptance items are met, and this goal adds no stage for them.**
  - The *Verification* fixtures of ADRs 0014, 0023, 0028, 0046 and 0069 are all on disk. That includes
    the `PropertyObserver` no-overhead guard
    (`benches/abi-probe/tests/perf_guards.rs:1108`, run in the floor with `--release`) and `E0467` for
    array `+`/`+=` (`tests/conformance/lang/array-plus-is-a-compile-error.nvst`).
  - "A non-trivial program leaks nothing under Valgrind" is the driver's valgrind sweep over the
    floor's `files` (`tools/loop.py:2444-2476`). That sweep runs `examples/objects.nvs`,
    `examples/control.nvs` and `examples/cycles.nvs` every goal.
  - The stale prose in `docs/plan/m4.md` (its 1000-case figure and `done*`) belongs to goal
    `plan-truth`.
- **Not this goal:**
  - every other gap in `crates/nvs-ir/src/lib.rs`: gap 7 is M12's, and the `unowned` rest belong to goal
    `unowned-closures`;
  - converting the engine invariants `holes.py` already does not count;
  - replacing `REFUSAL` with a source-declared kind over every site;
  - `nvs-codegen`, which holds no site today.

  A session that finds one of these on its path writes it to the handoff's `## Backlog`, or to
  carried-gaps.md if it will outlive this goal.
