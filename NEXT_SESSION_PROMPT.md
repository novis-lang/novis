# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session was again docs-only — planning, not coding — and touched nothing in
`mwl-ir`/`mwl-types`/any crate.** It added [ADR 0041](docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md),
amending [ADR 0018](docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md): trace
events gain a `call`/`gc`/`spawn` `kind` tag; the cycle collector's run routine and the three
isolate-spawn/join routines (M5) each get their own instrumentation point — deliberately *not* the
safepoint poll or any per-statement/per-call site, so it costs nothing on ADR 0018's already-measured hot
path; and a speedscope-evented export renders all three kinds as one scrollable timeline, reusing the open
format [ADR 0040](docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md) already committed to for the
sampling profiler rather than building a bespoke viewer. Two things raised in the same discussion were
explicitly *not* pursued — named in ADR 0041's own *Revisiting*: an external/live attach mechanism (a new
network-reachable trust boundary against priority 1, needing its own threat-modeled ADR if ever wanted),
and a memory/allocation timeline (no probe mechanism exists for it at all). ADR 0040/M4B stand exactly as
before this session. None of this changes anything about M2's in-progress `mwl-ir` work below. If picking
up coding work next, resume M2 exactly where the prior *coding* session left it, below.

**The most recent *coding* session landed array-element access (`$arr[$i]`, both read and write) — the item
the plan had flagged as needing a real design decision about missing-key behavior, and it resolved to "no
decision needed this slice" once the actual question was pinned down.** The guidance going in suspected
`mwl_types` has no compile-time "is this key present" concept at all, only an element-type resolution —
that turned out to be exactly right: `mwl_types::expr::check_expr`'s `ExprKind::Index` arm resolves the
same element type regardless of whether a given key exists at runtime, mirroring
`check_property_access`'s shape/`object`-erasure case (which also answers "what type" without ever
asking "does this exist"). So the missing-key runtime behavior (PHP's own warning-and-`null` read,
autovivification on write) was never actually in scope to weigh a design against — it's deferred
wholesale, the same way every other checked-throw already is in this crate (no `try`/`throw` lowering
exists at all yet). What *was* an actual design point, and got a real (if small) decision: `InstKind::
ArraySet` does **not** mirror `InstKind::FieldSet`'s "read the old value back with a `FieldGet`, then
release it" shape. A class field always exists once its instance is definitely initialized (ADR 0022), so
`FieldSet` can safely assume there's an old value to read and release. An array key may or may not already
be present — an *ordinary* `$arr[$newKey] = v;` insert is completely normal, not an edge case — so a
conditional get here would mean modeling the exact "does this key exist" question this slice just
deferred. `ArraySet` instead bundles the whole replace-or-insert into one instruction, leaving "release
whatever was there, if anything" as an implementation detail of `mwl-codegen`'s own future array-mutation
primitive, the same "no codegen exists yet to make this split observable" reasoning `InstKind::New`
already uses for allocation-plus-constructor.

**What landed, concretely:**

- `mwl_types::expr_table::ExprInfo::Index { elem_ty: TypeId }` (`crates/mwl-types/src/expr_table.rs`) — a
  new entry, the same shape as `ExprInfo::Property` minus a declaring class (an array has no class
  identity to name). Recorded by `crates/mwl-types/src/expr.rs`'s `ExprKind::Index` arm exactly when the
  base statically resolved to a known `Ty::Array(elem)`, left unrecorded when it erased to `mixed` — the
  same split `check_property_access` already draws for a shape/plain-`object` receiver. Recorded for a
  read and a write alike: `check_assign`'s general (non-plain-local) arm routes an assignment target back
  through the same `check_expr`/`Index` path a read takes, so both are keyed by the `Index` expression's
  own span.
- `ir::InstKind::ArrayGet { array: ValueId, key: ValueId }` (`crates/mwl-ir/src/ir.rs`) — reads `array` at
  `key` (already `Ty::Str`). Reads `array` without retaining it, same as `FieldGet` reads its `object`.
  Models only the happy path — no missing-key behavior at all, per the design note above.
- `ir::InstKind::ArraySet { array: ValueId, key: ValueId, value: ValueId }` — writes `value` at `key` into
  `array`, bundling replace-or-insert into one instruction rather than `FieldSet`'s get/release pair (see
  above for why). Defines no value.
- `lower::Lowering::lower_array_key` (`crates/mwl-ir/src/lower.rs`) — lowers an `Index`'s subscript and
  normalizes it to a `Ty::Str` key: ADR 0007 § 5's "every key is a `string`" rule, with an `int`/`uint`
  subscript converted to its decimal-string form via the **existing** `Helper::IntToString`/
  `Helper::UintToString` (no new `Helper` variant needed — the exact conversion `concat_operand` already
  had for `.`'s scalar operand, reused verbatim). Returns `(ValueId, bool)` mirroring `concat_operand`'s
  shape: the bool says whether the key value is itself an aliasing read of a durable slot (a plain `Ty::Str`
  local/property/array read passed through unchanged) versus a fresh, single-owner buffer (a converted
  `int`/`uint`, or any other fresh producer) — this drives the retain-on-write / release-after-read policy
  below. A `float`/`bool`/`null` subscript — the three source types ADR 0007 § 5 itself rejects as a key,
  which `mwl_types` doesn't yet enforce either (the same known gap `InstKind::ArrayNew`'s own doc comment
  already names for an array literal's explicit `key =>`) — panics naming the case.
- `lower::Lowering::lower_expr`'s new `ExprKind::Index` read arm: looks up `ExprInfo::Index` (panicking,
  naming the `mixed`-erasure/missing-table case, if absent), lowers the base and the key, emits
  `ArrayGet`, and releases the key right after when `lower_array_key` reported it's *not* an aliasing read
  (nothing else will ever release a freshly converted key) — the same "release a fresh value once its one
  and only use is done" policy `Concat`'s caller already applies. `base[]` (`index` is `None`, PHP's append
  syntax) panics — it is legal to *parse* in a read position (the parser's postfix-index loop doesn't
  restrict an empty subscript to assignment targets, and `mwl_types::expr::check_expr` doesn't reject it
  as a read either), so this needed its own explicit panic rather than being unreachable.
- `lower::Lowering::lower_reassignment`'s new `ExprKind::Index` target arm: looks up `ExprInfo::Index` at
  `target.span`, lowers the base and the key (retaining the key first if `lower_array_key` reported it's
  an aliasing read — the array now durably owns a second reference), lowers the value (retaining it too
  when refcounted and an aliasing read, the ordinary `bind_local`-style judgment), and emits `ArraySet` —
  no old-value get/release pair, per the design note above. `base[] = expr;` panics naming the gap (needs
  a "next available integer key" counter this crate has no representation for yet).
- `lower::is_aliasing_read` gained `ExprKind::Index` alongside `ExprKind::Variable`/`ExprKind::PropertyAccess`
  — an array-element read borrows the same "storage some other binding still owns" reference a property
  read does (ADR 0007 § 5's copy-on-write value semantics), so `bind_local`/`lower_call_args`/
  `StmtKind::Return`'s existing retain call sites picked this up with **no new insertion point at all** —
  the same "extend the judgment, not the call sites" pattern the `bytes`/`array<T>` slices already
  established.
- `print::print_inst` gained `InstKind::ArrayGet`/`InstKind::ArraySet` rendering (`array.get v1, v2` /
  `array.set v1, v2, v3`).

**Eight new tests.** Two in `crates/mwl-types/src/expr_table.rs`'s `tests` module
(`an_array_index_through_a_known_element_type_records_the_element_type`,
`an_array_index_through_a_mixed_base_records_nothing`) proving `ExprInfo::Index` is recorded/not-recorded
exactly when expected — `mwl-types` is now at 164 tests. Six new snapshot tests in
`crates/mwl-ir/src/lower.rs`'s `tests` module, reviewed by hand against the retain/release bookkeeping
they're meant to prove correct — `mwl-ir` is now at 62 tests:

- `reading_an_int_element_through_a_literal_key_normalizes_it_to_a_string` — `$a[0]` through an
  `array<int>` parameter: the literal key converts via `Helper::IntToString` and is released right after
  the `ArrayGet` reads it (fresh, non-aliasing); the `int` result needs no retain (not refcounted).
- `reading_a_string_element_through_a_string_local_key_retains_the_result` — `$a[$k]` through
  `array<string>`/`string` parameters: the key is a bare local read, so it's *not* released after the get
  (its own slot still owns it); binding the `string` result to `var $s` retains it (a new aliasing-read
  shape via `is_aliasing_read`'s `Index` arm).
- `reading_base_append_syntax_is_still_out_of_scope` / `writing_base_append_syntax_is_still_out_of_scope`
  / `a_bool_subscript_key_is_still_out_of_scope` — three `should_panic(expected = "known gaps")` tests for
  the three deliberately-out-of-scope shapes named above.
- `writing_an_int_element_through_a_literal_key_normalizes_it_to_a_string` — `$a[0] = 5;`: no retain of
  either the converted key or the fresh `int` value, just `ArraySet` and the array's own exit release.
- `writing_a_string_element_through_a_string_local_key_retains_both_key_and_value` — `$a[$k] = $v;` where
  both are `string` locals: both get retained before `ArraySet` (the array now durably owns a second
  reference to each), and all three locals (`$a`/`$k`/`$v`) still get their ordinary exit-sweep release.

`cargo build`/`test`/`clippy --all-targets -- -D warnings`/`fmt --check` all clean across the whole
workspace. `crates/mwl-ir/src/lib.rs`'s module docs (the "what this crate lowers so far" list, the
design-choices section, the known-gaps section), `crates/mwl-ir/src/ty.rs`'s `Ty::Array` doc comment, and
`crates/mwl-ir/src/ir.rs`'s `InstKind`/`Helper` doc comments were all updated in place (not appended) to
describe the slice; `docs/implementation-plan.md`'s M2 paragraph was updated the same way — note that
section was *already* over `.claude/brief.py`'s 4000-byte budget before this session (it truncates and
says so); it grew slightly more this session since the edit was additive-in-place rather than a trim. Not
fixed here — `DOC_CLEANUP_PROMPT.md`'s trim pass is the user-run remedy for that, unrelated to this
session's own scope.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, a compile-time-known property access, and array-element
   access through a known `int`/`uint`/`string` key.~~ **Done.** What's left of this shape:
   - **`$a[]`/`$a[] = expr;` (PHP's append syntax).** Needs a "next available integer key" counter this
     crate has no representation for yet — genuinely more than mechanical, since it means tracking (or
     re-deriving) an array's own highest-inserted-integer-key state at lowering time, not just reading one
     back.
   - **A `float`/`bool`/`null` array-subscript key.** `ADR 0007 § 5` rejects these outright as a key
     source type, but `mwl_types::expr::check_expr`'s `Index` arm doesn't enforce that yet — `mwl-ir`
     panics naming the case in `lower_array_key` rather than guessing at a conversion PHP itself doesn't
     define. Fixing this properly means a `mwl_types` checker-side diagnostic first (same shape as item 4's
     array-literal explicit-key gap below), not an `mwl-ir` change.
   - **Array-element access through a `mixed`-erased base.** No `ExprInfo::Index` entry exists for that
     case (mirrors `ExprInfo::Property`'s shape/`object`-erasure gap), so lowering panics naming it — but
     this is currently *unreachable* without first hitting the unrelated, already-documented "no
     `Ty::Mixed` representation" gap (item 5 below), since `mixed` isn't a lowerable declared type or
     resolved-call return type in this crate yet either. No dedicated fixture for it this session for that
     reason; add one once `Ty::Mixed` lands if it's still worth a dedicated proof at that point.
4. **Non-scalar *data* values and refcount operations — `string`/`bytes`/`array<T>` locals, the
   call/return/property-read-and-write boundary, `.` concatenation, and now array-element read/write are
   all landed; two pieces remain, both mechanical:**
   - **An explicit `key =>`, a `...spread`, or a `&value` array-literal element.** Unchanged from before:
     `mwl_types::expr::check_array_literal` itself has no key-normalization/rejection logic yet (ADR 0007
     § 5's int/uint-to-decimal-string normalization, float/bool/null rejection), so lowering an explicit
     key would mean guessing at a runtime conversion this crate can't yet synthesize. Landing this probably
     wants a checker-side fix first (`mwl-types`), not just an `mwl-ir` change — and, now that
     `lower_array_key`'s int/uint-to-string conversion exists, is likely to reuse it once the checker side
     is ready.
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — the
     `bind_local`/`lower_call_args`/`release_all_locals`/`lower_expr_stmt`/`lower_reassignment` insertion
     points already extended for `Ty::Str`/`Ty::Bytes`/`Ty::Array` are expected to extend to it directly
     (just flip `Ty::is_refcounted` to include `Ty::Object` and re-run the existing test suite to see what
     breaks), once there's an actual allocation/field-layout story to attach it to — check whether one
     exists yet before assuming it's ready, per this loop's own standing instructions.
   - **Qualified string/bytes types (`tainted`, `secret`, and their combination).** `lower_checked_ty` only
     has arms for the plain `CheckedTy::String`/`CheckedTy::Bytes`/`CheckedTy::Array`; the six qualified
     string/bytes variants (`TaintedString`/`SecretString`/`SecretTaintedString` and their `Bytes`
     counterparts) still panic. These likely want to wait for ADR 0024 §4/0033's stdlib-dependent sinks
     anyway (M7/M8), since a qualifier with nothing to launder against isn't very actionable yet.
5. **Runtime-helper calls are landed** (`ir::InstKind::HelperCall`/`ir::Helper`), now used for both `.`'s
   scalar-to-`string` conversion and an `int`/`uint` array-subscript's key normalization. Two more named
   uses remain, both blocked on something other than the `HelperCall` shape itself now:
   - **A `mixed`/union operand** — needs a `Ty::Mixed`-shaped IR representation first; none exists yet, so
     there's nothing for a helper to dispatch on. Adding one is its own small design question (how a
     `mixed` value's runtime type tag is represented) before any helper call can use it. (This is also
     what item 3's "mixed-erased array base" gap above is blocked on.)
   - **ADR 0035's truthy conversion** for a non-`bool` `if`/`while` condition — PHP's truthy table differs
     by source type (`0`/`0.0`/`""`/`"0"`/an empty array/`null` are falsy, everything else truthy). Both
     `Ty::Array` and array-element access exist now, giving the array-emptiness case a representation to
     convert *from* — but a nullable-type representation still doesn't, so the `null` case still waits. A
     scalar-plus-array-only truthy helper could land now as a partial slice if a fixture wants it.
   - Both are expected to add new `Helper` variants to the same enum, not a second call-shaped instruction.
   - The one remaining `.`-concatenation gap — a `Stringable`-object operand — is *not* primarily a
     `HelperCall` gap any more: it needs `.` to synthesize a resolved `toString()` call, which needs either
     a checker-side change (recording an `ExprInfo::Call`-shaped resolution for a `.` operand, not just a
     call expression) or this crate re-resolving it independently. See the "runtime-helper calls" session's
     design-choices writeup in `mwl-ir`'s module docs before picking this up — it's a small but genuine
     decision, not a mechanical extension.
6. **Virtual dispatch** — every call/access lowered so far (`new`'s constructor, a static call, an
   instance call, a property access, an array-element access) has its receiver's *static* type equal to
   its *runtime* class — none has gone through an interface-typed or overridden-method/property receiver
   yet, which is the first place the two could actually differ. Whether a real vtable/interface-dispatch
   lookup belongs at this IR level (as opposed to purely at codegen, once M3 exists) is an open question
   for whichever session first hits that shape.
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

**Separately, whenever M3 finishes and M4 is underway:** keep [ADR 0040](docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md)
in mind as M4 approaches its own "usable CLI language" exit criterion — M4B (minimal `mwl-lsp` +
`editors/vscode`, plus `mwl-syntax`'s new resilient-parse mode) starts right after, per the plan.

**Separately, whenever M5 (concurrency and script isolates) is underway:** give each of the three
spawn-construct runtime routines its `spawn`-kind trace hook, and whenever the mark-sweep cycle collector's
run routine is built, give it its `gc`-kind hook too — both per [ADR 0041](docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md),
both instrumentation-only inside those already-rare routines, no change to the safepoint poll itself.
