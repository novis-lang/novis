# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed item 4 from the list below — `string` locals with refcount retain/release
operations.** This is the milestone text's first non-scalar *data* representation and the first refcount
operations of any kind to land in `mwl-ir`, so it's worth reading carefully before widening further:

- `ty::Ty` gained `Ty::Str` (a reference-counted, heap-allocated `string`) plus `Ty::is_refcounted()`,
  currently `true` only for `Ty::Str`. `ir::InstKind` gained three variants: `ConstStr(String)` (a
  literal's cooked bytes — a fresh value with exactly one natural owner), `Retain { operand }` and
  `Release { operand }` (both side-effecting, define no value; `Release` is reserved/inert the same
  "shape now, functional once a backend exists" way `InstKind::Safepoint` already is).
- `lower::lower_decl_type` (a type spelled directly in source) now maps `TypeAtom::String` to `Ty::Str` —
  so a `string`-typed local, parameter, or method return type all lower. `lower::lower_checked_ty` (a
  *resolved* call's/`new`'s parameter/return type, or a resolved property's field type) was deliberately
  **not** touched, so a call/`new` with a `string` argument, a call whose return type is `string`, and a
  `string`-typed property read/write all still panic naming `string` unsupported — see below.
- `lower::cook_str_literal` cooks a plain (non-interpolated) `ExprKind::Str` literal into its runtime
  bytes: a single-quoted literal only unescapes `\\`/`\'` (the two escapes `mwl-syntax`'s lexer actually
  recognizes there); a double-quoted one additionally unescapes `\n`/`\t`/`\r`/`\\`/`\"`/`\$`/`\0`. A
  numeric escape (`\xHH`, `\u{...}`, octal) passes through literally rather than cooking — a known gap,
  same style as the existing "integer literal magnitude not checked" gap. `ExprKind::Interpolated` and a
  heredoc/nowdoc-sourced `ExprKind::Str` are both unsupported, panicking naming the gap.
- **The refcount insertion policy — the actual design question this slice had to answer — is naive and
  syntactic, not a liveness/move analysis.** Two designs were weighed (see `lib.rs`'s design-choices
  section for the full writeup): (a) a full last-use/move analysis that only retains when a value is
  genuinely shared, or (b) inserting a retain everywhere a value is copied into a second durable slot and
  a release everywhere a slot's value is overwritten or the function exits, with no attempt to prove a
  copy was unnecessary. (b) was chosen — nothing can execute this IR yet to make (a)'s payoff measurable,
  `docs/implementation-plan.md`'s optimizer feature list already names "refcount elision" as separate
  future work (so naive-then-elide was already the plan's own intent, not a new call), and generalizing
  ADR 0004/0006's isolate-boundary "move when refcount is 1" optimization to ordinary lowering here would
  be scope creep beyond what any ADR asks for. Concretely: `Lowering::bind_local` retains a value read
  out of an *existing* `$name` binding (`ExprKind::Variable`) when it's copied into another durable slot
  — a fresh literal needs no retain, since it already has exactly one natural owner and the bind just
  names it. A slot's *previous* value is released on overwrite, and `Lowering::release_all_locals`
  releases every refcounted local still live at a `return`/implicit-`void` fallthrough — except the one
  slot whose value is the return expression itself, when that expression is a bare `$name` read (it
  transfers out instead of being copied-then-dropped). This stays exactly balanced today only because a
  `string` value currently has exactly two possible producers (a fresh literal, or a bare-variable copy)
  — widening `lower_checked_ty` to let a call argument/return/property field produce or consume one will
  need the same "is this a borrow of storage someone else still owns" judgment extended to a property
  read (which needs retain-on-copy treatment, same as a bare variable read) and to whatever calling
  convention a call/return boundary picks (does the caller retain before passing and the callee release
  at its own exit, mirroring what a local already does?).
- `print.rs` renders `const.str "..."` (Rust `Debug` escaping), `retain vN`, `release vN`.

Four new `insta` snapshot tests landed, all under `crates/mwl-ir/src/lower.rs`'s `tests` module:
`string_literals_cook_their_escapes_and_release_at_scope_exit` (two unrelated `string` locals, no
aliasing — exercises literal cooking of both quote kinds and the plain exit-sweep release, no retains at
all), `assigning_one_string_local_to_another_retains_the_shared_value` (`string $b = $a; return $b;` —
exercises the alias-retain and the return-exclusion together: exactly one retain, one release, never zero
or two), and `reassigning_a_string_local_releases_its_previous_value` (`$x = "a"; $x = "b";` — exercises
the overwrite-release path). Read the actual snapshot output before trusting the design holds: it does
(verified this session) — `mwl-ir` is now at 25 tests (`mwl-types` unchanged at 162). `cargo build`/
`test`/`clippy --all-targets -- -D warnings`/`fmt --check` all clean across the whole workspace.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, and a compile-time-known property access (including the
   shape/`object`-erasure panic case).~~ **Done.** One shape remains:
   - **Array access (`$arr[$i]`)** is still unsupported; lowering panics naming the expression. No
     `array<T>` *data* representation exists yet either (see item 4 below), so this may naturally land
     together with that slice rather than alone.
4. **Non-scalar *data* values and refcount operations — `string` locals landed this session; three
   pieces remain, roughly in this order:**
   - **`string` crossing a call-argument/return/property-field boundary.** `lower_checked_ty` still
     panics naming `CheckedTy::String`. This needs the calling-convention question the design-choices
     writeup above flags as still open: does the caller retain a `string` argument before passing it and
     the callee release it at its own exit (symmetric with what a local already does), or does ownership
     move some other way? A property read also needs the same "this borrows storage someone else still
     owns" retain-on-copy treatment a bare variable read already gets in `bind_local` — it isn't one
     today only because `lower_checked_ty` blocks a `string`-typed field from ever reaching `FieldGet` in
     the first place.
   - **String concatenation (`.`).** `ExprKind::Binary`'s arm only accepts the scalar
     arithmetic/equality/ordering operators; `BinaryOp::Concat` on two `string` operands panics there.
     Lowering this needs a runtime-helper call (see item 5) or a dedicated `InstKind`, since concatenation
     allocates a new buffer rather than being a native scalar instruction.
   - **`bytes`.** Expected to be a mechanical repeat of `Ty::Str`'s shape (same refcounted-heap-value
     treatment, different content, same `bind_local`/`release_all_locals` insertion points) — extend
     `Ty::is_refcounted`, `lower_decl_type`, and add an escape-cooking helper once a fixture needs one.
   - **`array<T>`.** Needs its own element-layout decision first (this is the one still-open piece with a
     real design tradeoff — contiguous vs. hashmap-backed storage, COW-on-write semantics per ADR 0004 —
     work out the tradeoff explicitly before implementing, per CLAUDE.md's priority ordering).
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — the
     `bind_local`/`release_all_locals` insertion points this session built are expected to extend to it
     directly (just flip `Ty::is_refcounted` to include `Ty::Object` and re-run the existing test suite to
     see what breaks), once there's an actual allocation/field-layout story to attach it to.
5. **Runtime-helper calls** — the milestone's fourth named ingredient, needed for a `mixed`/union operand,
   for ADR 0035's truthy conversion on a non-`bool` `if`/`while` condition, and now also for string
   concatenation (see item 4).
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
   whenever convenient — likely wants to happen alongside string concatenation (item 4) since
   interpolation desugars to concatenation-like codegen anyway.

Once control flow, calls, and property/array access all lower, M2's own *Verify* bullet ("IR snapshot
tests; no program in the corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven
snapshot suite, not just hand-written fixtures — at that point M2 as a whole should be closeable and M3
(baseline Cranelift backend, `Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).

---

Pick the best-scoped next item and land it end to end (design decision if needed → implementation → tests
→ verification → docs → commits). If you pick up `array<T>`'s element-layout decision (flagged above as a
real tradeoff), work out the design question explicitly (1-2 paragraphs weighing the options against
CLAUDE.md's priority ordering: security > correctness > latency > simplicity > memory) before writing
code, and stop to report back if it's genuinely a decision only the user should make rather than a
mechanical extension of an existing pattern. Good luck.
