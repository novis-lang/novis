# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed `bytes` — the mechanical follow-on to `string`'s own shape that the plan had flagged
two sessions running as the most likely "no design decision needed" pickup, and it turned out to be exactly
that.** `Ty::Bytes` (`crates/mwl-ir/src/ty.rs`) is a second refcounted, heap-allocated representation
alongside `Ty::Str`. Landing it needed no new `Lowering` insertion point at all — every retain/release site
(`Lowering::bind_local`, `Lowering::lower_call_args`, `Lowering::release_all_locals`,
`Lowering::lower_reassignment`'s property-target arm) already keys off `Ty::is_refcounted`/
`lower::is_aliasing_read` rather than naming `Ty::Str` directly, so the only code changes were:

- `Ty::is_refcounted` now matches `Ty::Str | Ty::Bytes`.
- `lower::lower_decl_type` gained a `TypeAtom::Bytes => Ty::Bytes` arm (declared types read straight off
  the AST — parameters, locals, return types).
- `lower::lower_checked_ty` gained a `CheckedTy::Bytes => Ty::Bytes` arm (a resolved call's/`new`'s
  parameter/return type, or a property's field type, read back from `mwl_types::expr_table::ExprTypeTable`).
- `print::ty_name` gained a `Ty::Bytes => "bytes"` arm, and `Lowering::concat_operand`'s inner
  helper-dispatch match (over `Ty::Bool`/`Int`/`Uint`/`Float`, with `Ty::Str`/`Ty::Void`/`Ty::Object`
  `unreachable!`) had to add `Ty::Bytes` to that `unreachable!` arm too, since `Ty` is matched exhaustively
  within this crate (`#[non_exhaustive]` only restricts *other* crates).

**One asymmetry with `string`, worth knowing before writing a `bytes` fixture**: `mwl-syntax`'s grammar has
*no* `bytes` literal syntax at all — no `b"..."` form or equivalent, confirmed by grepping the lexer/parser/
AST. So unlike `string`'s `ExprKind::Str` → `InstKind::ConstStr`, nothing constructs a *fresh* `bytes` value
from a literal. Every new test sources its `bytes` value from a parameter or a compile-time-known property
read instead — both already-covered `is_aliasing_read` shapes — e.g. a `Foo` class with a `bytes` property
initialized from a constructor parameter (`public bytes $data; function constructor(bytes $data) { $this->data
= $data; }`) rather than the existing `string` tests' `public string $name = "hi";` literal-defaulted
pattern. This isn't a gap this slice left open (there's nothing more to build — a `Core\Bytes` conversion/
constructor, once one exists in M7/M8, would just be the first fresh producer, using the exact same
representation already landed); it's just why the five new tests below look structurally different from
their `string` counterparts despite exercising the identical policy.

Five new snapshot tests in `crates/mwl-ir/src/lower.rs`'s `tests` module, all passing and reviewed by hand
against the retain/release bookkeeping they're meant to prove correct:

- `a_bytes_parameter_bound_to_a_local_transfers_out_on_return` — mirrors
  `assigning_one_string_local_to_another_retains_the_shared_value`: `bytes $b = $a;` retains the parameter's
  value, `return $b;` transfers it out untouched, leaving one release for `$a`'s own slot.
- `reassigning_a_bytes_local_retains_the_new_value_and_releases_the_old` — a two-parameter reassignment
  (`bytes $x = $a; $x = $b;`), since there's no literal to reassign to. Worth reading closely: `$a`'s
  storage ends up released twice in the printed IR (once when `$x` moves off it, once for `$a`'s own slot
  at exit) and so does `$b`'s (once for `$b`'s own slot, once for `$x`, which now aliases it) — that's
  correct, not a double free, because two live slots really did hold independent references to each value
  at different points. The doc comment on the test walks through the refcount arithmetic in full.
- `passing_a_bytes_local_as_a_call_argument_retains_it` — mirrors the `string` analog exactly.
- `binding_a_bytes_property_read_to_a_local_retains_it` / `writing_a_bytes_local_to_a_property_retains_it_before_releasing_the_old_value`
  — mirror the `string` property read/write tests, using the constructor-initialized `Foo` pattern above.

`mwl-ir` is now at 47 tests (`mwl-types` unchanged at 162). `cargo build`/`test`/`clippy --all-targets -- -D
warnings`/`fmt --check` all clean across the whole workspace. `crates/mwl-ir/src/lib.rs`'s module docs and
`ty.rs`'s own doc comment on `Ty::Bytes` were updated in place (not appended) to describe the slice and its
one asymmetry; `docs/implementation-plan.md`'s M2 paragraph was updated the same way — note that section
was *already* over `.claude/brief.py`'s 4000-byte budget before this session (it truncates and says so); it
grew slightly more this session since the edit was additive-in-place rather than a trim. Not fixed here —
`DOC_CLEANUP_PROMPT.md`'s trim pass is the user-run remedy for that, unrelated to this session's own scope.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, and a compile-time-known property access (including the
   shape/`object`-erasure panic case).~~ **Done.** One shape remains:
   - **Array access (`$arr[$i]`)** is still unsupported; lowering panics naming the expression. No
     `array<T>` *data* representation exists yet either (see item 4 below), so this may naturally land
     together with that slice rather than alone.
4. **Non-scalar *data* values and refcount operations — `string`/`bytes` locals, the
   call/return/property-read-and-write boundary, and `.` concatenation (including a scalar operand, via a
   helper call) are all landed; two pieces remain:**
   - **`array<T>`.** Needs its own element-layout decision first — **this is now the one remaining piece
     with a real design tradeoff** (contiguous vs. hashmap-backed storage, COW-on-write semantics per ADR
     0004 — work out the tradeoff explicitly before implementing, per CLAUDE.md's priority ordering). With
     `bytes` now landed as the last purely-mechanical scalar-shaped widening, `array<T>` is the natural next
     pickup for a session that wants to make a real design call rather than another mechanical extension.
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — the
     `bind_local`/`lower_call_args`/`release_all_locals`/`lower_expr_stmt`/`lower_reassignment` insertion
     points already extended for `Ty::Str`/`Ty::Bytes` are expected to extend to it directly (just flip
     `Ty::is_refcounted` to include `Ty::Object` and re-run the existing test suite to see what breaks),
     once there's an actual allocation/field-layout story to attach it to — check whether one exists yet
     before assuming it's ready, per this loop's own standing instructions.
   - **Qualified string/bytes types (`tainted`, `secret`, and their combination).** `lower_checked_ty` only
     has arms for the plain `CheckedTy::String`/`CheckedTy::Bytes`; the six qualified variants
     (`TaintedString`/`SecretString`/`SecretTaintedString` and their `Bytes` counterparts) still panic.
     These likely want to wait for ADR 0024 §4/0033's stdlib-dependent sinks anyway (M7/M8), since a
     qualifier with nothing to launder against isn't very actionable yet.
5. **Runtime-helper calls are landed** (`ir::InstKind::HelperCall`/`ir::Helper`), but only for `.`'s
   scalar-to-`string` conversion. Two more named uses remain, both blocked on something other than the
   `HelperCall` shape itself now:
   - **A `mixed`/union operand** — needs a `Ty::Mixed`-shaped IR representation first; none exists yet, so
     there's nothing for a helper to dispatch on. Adding one is its own small design question (how a
     `mixed` value's runtime type tag is represented) before any helper call can use it.
   - **ADR 0035's truthy conversion** for a non-`bool` `if`/`while` condition — PHP's truthy table differs
     by source type (`0`/`0.0`/`""`/`"0"`/an empty array/`null` are falsy, everything else truthy), and
     most of those source types (an array, a nullable value) have no IR representation to convert *from*
     yet either. A scalar-only truthy helper (`int`/`uint`/`float`/`string`/`bool`/`bytes` — reusing
     `Ty::Str`/`Ty::Bytes` for the `""`/`"0"` case) could land now as a partial slice if a fixture wants it;
     the array/`null` cases wait on item 4's `array<T>` and a nullable-type representation, neither of
     which exist yet.
   - Both are expected to add new `Helper` variants to the same enum, not a second call-shaped instruction.
   - The one remaining `.`-concatenation gap — a `Stringable`-object operand — is *not* primarily a
     `HelperCall` gap any more: it needs `.` to synthesize a resolved `toString()` call, which needs either
     a checker-side change (recording an `ExprInfo::Call`-shaped resolution for a `.` operand, not just a
     call expression) or this crate re-resolving it independently. See the "runtime-helper calls" session's
     design-choices writeup in `mwl-ir`'s module docs before picking this up — it's a small but genuine
     decision, not a mechanical extension.
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
   written-out `.` expression already lowers to, so this pairs naturally with that, not with a separate
   mechanism.

Once control flow, calls, and property/array access all lower, M2's own *Verify* bullet ("IR snapshot
tests; no program in the corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven
snapshot suite, not just hand-written fixtures — at that point M2 as a whole should be closeable and M3
(baseline Cranelift backend, `Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).
