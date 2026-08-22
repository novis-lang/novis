# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed `.` string concatenation between two already-`string` operands — the item flagged as
the most natural next pickup two sessions ago.** One new `InstKind` was needed:

- `crate::ir::InstKind::Concat { lhs, rhs }` — builds a fresh `Ty::Str` value from two already-lowered
  `Ty::Str` operands. Modeled as a dedicated instruction rather than a runtime-helper call: no
  runtime-helper-call shape exists in the IR yet (that's still item 5 below, unstarted), and `.` only ever
  needs this one fixed two-operand shape, the same "native instruction over already-typed operands"
  treatment `InstKind::BinOp` already gives scalar arithmetic. `crate::print::print_inst` renders it as
  `concat v{lhs}, v{rhs}`.
- `Lowering::lower_expr`'s `ExprKind::Binary` match gained a dedicated arm for `BinaryOp::Concat`, ahead of
  the existing scalar-operator table, so it never reaches that table's catch-all panic. Both operands lower
  with `expected: Some(Ty::Str)`; if either doesn't come back `Ty::Str` (a bare `int`/`uint`/`float`/`bool`
  literal, or any other operand shape), lowering panics naming the mismatch rather than guessing a
  conversion — confirmed against `mwl_types::expr::check_expr`'s own `BinaryOp::Concat` handling
  (`require_stringable`), which *does* accept a scalar or a `Stringable`-implementing object on either side
  (PHP-style implicit stringification) and resolves the result to a plain `string` (poisoned `tainted`/
  `secret` if either operand already was, per ADR 0024 §2/0033 §2) — so the checker is strictly more
  permissive here than this slice's lowering; a scalar/`Stringable` operand is a known, named gap, not a
  checker/lowering mismatch bug.
- **No retain of either operand, and no retain of the result.** Concatenation only *reads* each operand to
  build a new buffer — it never becomes a second durable owner of either, the same reasoning
  `InstKind::FieldGet` already uses for not retaining its `object` receiver. The result is a fresh value
  with exactly one natural owner, same as `ConstStr`/`New`/a call's result — `Lowering::is_aliasing_read`
  needed no new arm for `ExprKind::Binary` (it was already `false` by omission). Verified by hand in the new
  snapshots: two string locals concatenated and immediately discarded produce exactly one release per
  original local plus one release for the concat's own result — three releases, zero retains, for a
  function with three `string`-typed bindings.
- Two new `insta` snapshot tests in `crates/mwl-ir/src/lower.rs`'s `tests` module:
  `concatenating_two_string_literals_needs_no_retain_of_either_operand` (`return "a" . "b";` — two
  `const.str` values feeding one `concat`, returned with no retain at all) and
  `concatenating_two_string_locals_reads_them_without_retaining` (`string $a = "x"; string $b = "y"; string
  $c = $a . $b;` — confirms no retain on the way into `concat`, and exactly one release per slot at the exit
  sweep). A third test, `concatenating_a_non_string_operand_is_still_out_of_scope` (`#[should_panic]`, `1 .
  "x"`), pins the scalar-operand panic. Read the actual snapshot output before trusting the design holds: it
  does (verified this session, both non-panic snapshots checked by hand) — `mwl-ir` is now at 40 tests
  (`mwl-types` unchanged at 162). `cargo build`/`test`/`clippy --all-targets -- -D warnings`/`fmt --check`
  all clean across the whole workspace.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, and a compile-time-known property access (including the
   shape/`object`-erasure panic case).~~ **Done.** One shape remains:
   - **Array access (`$arr[$i]`)** is still unsupported; lowering panics naming the expression. No
     `array<T>` *data* representation exists yet either (see item 4 below), so this may naturally land
     together with that slice rather than alone.
4. **Non-scalar *data* values and refcount operations — `string` locals, the call/return/property-read-and-
   write boundary, and `.` concatenation between two `string` operands are all landed; three pieces remain:**
   - **`bytes`.** Expected to be a mechanical repeat of `Ty::Str`'s shape (same refcounted-heap-value
     treatment, different content, same `bind_local`/`lower_call_args`/`release_all_locals`/`lower_expr_stmt`/
     `lower_reassignment` insertion points, same `is_aliasing_read` reuse) — extend `Ty::is_refcounted`,
     `lower_decl_type`, `lower_checked_ty`, and add an escape-cooking helper once a fixture needs one.
   - **`array<T>`.** Needs its own element-layout decision first (this is the one still-open piece with a
     real design tradeoff — contiguous vs. hashmap-backed storage, COW-on-write semantics per ADR 0004 —
     work out the tradeoff explicitly before implementing, per CLAUDE.md's priority ordering).
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — the
     `bind_local`/`lower_call_args`/`release_all_locals`/`lower_expr_stmt`/`lower_reassignment` insertion
     points already extended for `Ty::Str` are expected to extend to it directly (just flip
     `Ty::is_refcounted` to include `Ty::Object` and re-run the existing test suite to see what breaks),
     once there's an actual allocation/field-layout story to attach it to.
   - **Qualified string/bytes types (`tainted`, `secret`, and their combination).** `lower_checked_ty` only
     has an arm for the plain `CheckedTy::String`; the four qualified variants
     (`TaintedString`/`SecretString`/`SecretTaintedString`, and their `Bytes` counterparts once `bytes`
     lands) still panic. These likely want to wait for ADR 0024 §4/0033's stdlib-dependent sinks anyway
     (M7/M8), since a qualifier with nothing to launder against isn't very actionable yet.
5. **Runtime-helper calls** — the milestone's fourth named ingredient, needed for a `mixed`/union operand,
   for ADR 0035's truthy conversion on a non-`bool` `if`/`while` condition, and for converting a scalar or
   `Stringable`-object operand to `string` for `.` concatenation (`mwl_types::expr::check_expr`'s
   `require_stringable` already accepts either; only the two-`string`-operand case lowers today — see item
   4's now-closed bullet above). This is probably the next high-leverage pickup: it's named by three
   separate gaps now, all blocked on the same missing IR shape.
6. **Virtual dispatch** — every call/access lowered so far (`new`'s constructor, a static call, an
   instance call, a property access) has its receiver's *static* type equal to its *runtime* class — none
   has gone through an interface-typed or overridden-method/property receiver yet, which is the first
   place the two could actually differ. Whether a real vtable/interface-dispatch lookup belongs at this IR
   level (as opposed to purely at codegen, once M3 exists) is an open question for whichever session first
   hits that shape.
7. ~~`var` locals (ADR 0037) and multi-base integer-literal cooking (hex/octal/binary).~~ **Done.** Full
   integer-literal *magnitude* range-checking (negative-into-`uint`, too-large-for-either) is still not
   modeled, mirroring `mwl_types::expr::infer`'s own documented gap for the same case — small and
   independent, land whenever convenient.
8. **String-literal cooking completeness** — a numeric escape (`\xHH`, `\u{...}`, octal) inside a
   double-quoted literal passes through uncooked rather than resolving to the byte/codepoint it names;
   `ExprKind::Interpolated` (a double-quoted string/heredoc with an interpolation site) and a
   heredoc/nowdoc-sourced `ExprKind::Str` are both entirely unsupported. Small and independent, land
   whenever convenient — interpolation is expected to desugar to the same `InstKind::Concat` chain a
   written-out `.` expression now already lowers to (item 4/this session's landing), so this pairs
   naturally with that, not with a separate mechanism.

Once control flow, calls, and property/array access all lower, M2's own *Verify* bullet ("IR snapshot
tests; no program in the corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven
snapshot suite, not just hand-written fixtures — at that point M2 as a whole should be closeable and M3
(baseline Cranelift backend, `Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).
