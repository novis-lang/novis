# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Also landed a couple of sessions back, independent of the `mwl-ir` thread below: [ADR 0039](docs/adr/0039-canonical-code-formatting.md)**
decides `mwl fmt`'s actual formatting rules — PER as the base style, explicit rules for the MWL-only
constructs PER never saw (`fn` closures, `tainted`/`secret`, `lateinit`, shape types, `match`), a
gofmt-style no-reflow model (never wraps/collapses an expression by width), zero configuration ever, and a
hard separation from the compiler (`mwl fmt --check` warns; `mwl check` never does). Docs only — `mwl-fmt`
itself doesn't exist until M10, so there is nothing to build from this yet.

**Last session landed `mwl-ir`'s control-flow slice** — `if`/`while`, the first item in the widening order
the previous prompt laid out. This is where SSA's real join/phi question got answered:

- **`crates/mwl-ir/src/lower.rs`** now has a `Lowering` builder that threads a `cur: &mut BlockId` through
  the recursive lowering calls: lowering a plain statement appends to that block; lowering `if`/`while`
  seals it with a real `Terminator::Branch`, lowers each arm into its own block, and hands back a new
  current block (the merge point for `if`, the loop's exit for `while`) for whatever comes next. Every
  block created is guaranteed to eventually get sealed — either by a `return` inside it, by an explicit
  `Jump` back to a merge/loop point, or, for the one block still open at the very end of the method body,
  by `lower_method`'s existing fallback `return;`.
- **`if`'s join and `while`'s loop-header join are each a single, hand-rolled two-predecessor (or
  pre-loop/back-edge) SSA merge** — not a general dominance-based phi-placement algorithm, since a
  structured `if`/`while` only ever produces that one join shape. A local keeping the same `ValueId` on
  every incoming edge needs no phi (SSA value numbering falls out for free, same as the straight-line
  slice); one that differs gets a fresh `ir::InstKind::Phi` (new variant, this session). A `while` header's
  phi is seeded with only its pre-loop incoming edge before the body is lowered (the back-edge value isn't
  known yet), then patched with the body's exit value afterwards — see `Lowering::lower_while`'s own doc
  comment. Which locals need a header phi at all comes from a syntactic pre-scan
  (`Lowering::collect_reassigned_locals`), not a second type-check — it only has to safely
  over-approximate "might be reassigned in the loop body", since a spurious phi is redundant, never wrong.
- **A name missing from some incoming environment at a join point is silently dropped from the merged one**
  rather than treated as an error: per `mwl_types::check_program`'s existing definite-assignment rule, any
  local actually used after the join must already be assigned on every path reaching it, so if it's really
  needed it will be present in every incoming environment by construction.
- **Determinism fix worth knowing about:** both merge sites originally iterated an `FxHashMap`/`FxHashSet`
  in its own bucket order, which would have made phi/id assignment depend on hash-table internals rather
  than source order alone — a real conflict with `ids.rs`'s own "an id is stable across recompiles of the
  *same* source" contract. Fixed before committing: `merge_envs` sorts local names before deciding which
  need a phi, and `collect_reassigned_locals` now collects into a `Vec` in first-occurrence source order
  (with an `FxHashSet` alongside only for O(1) dedup), not a hash set's iteration order.
- **An `if`/`while` condition must already be statically `bool`** — lowering panics naming this if not.
  ADR 0035's full truthy-table conversion for a non-`bool` condition needs a runtime-helper call, which
  doesn't exist in the IR yet (item 5 below).
- Six new snapshot tests (`crates/mwl-ir/src/lower.rs`'s `tests` module): an `if`/`else` merge needing a
  real phi, an `if` with no `else` (the implicit false edge lands straight on the merge block), both
  branches of an `if` always `return`ing (the merge block is dead but still needs a well-formed
  terminator — handled by falling back to the pre-branch environment rather than adding a dedicated
  "unreachable" terminator), a `while` loop carrying two pre-existing locals through header phis, and a
  `should_panic` proving `for` is still out of scope. `mwl-ir` is now at 9 tests (was 5); mwl-types stays at
  154, untouched. `cargo build`/`test`/`clippy --all-targets -- -D warnings`/`fmt --check` all clean across
  the whole workspace.
- **Known, documented gap, not fixed this session:** a nested `{}` `LocalDecl` that shadows an outer local
  of the same name is not distinguished from a reassignment of the outer binding — the environment is one
  flat, function-wide map with no notion of nested lexical scopes. Not observable for any program in scope
  today, but worth fixing (or at least re-checking) before this crate's `Env` is trusted with more shapes.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.** `for`/`switch`/`try`, plus `break`/`continue` of any kind, are
   still out of scope — reuse `merge_envs`/the seed-then-patch phi dance rather than inventing a new
   algorithm; `switch`'s fallthrough-by-omitted-`break` shape and `try`/`catch`'s exceptional edges are the
   two likely to need something beyond a straight port.
2. **Safepoints.** Belong at a loop back-edge and at function entry (recursion). `while` now lowers a real
   back edge (`Terminator::Jump` from the loop body to its header) — reserve the instruction/marker shape
   there and at function entry. No guard test needs this until M3, so this is a good next slice: small,
   self-contained, and doesn't need the design decision in (3) below.
3. **Calls, `new`, property/array access.** This is where the "does `mwl-ir` depend on `mwl-types`, or does
   `mwl-types` publish a typed-expression table" decision (carried forward from two sessions ago, still
   unresolved because nothing so far has forced it) has to actually be made — resolve it before writing the
   lowering code, not after. Not a session-ending blocker on its own, but flag it explicitly if you reach
   it: it's a real design choice between "duplicate a chunk of `mwl_types::expr`'s (currently `pub(crate)`)
   type-inference logic inside `mwl-ir`" and "give `mwl-types` a new, deliberately-designed public
   typed-expression table" — worth deciding once, not drifting into.
4. **Non-scalar values (`string`/`bytes`, arrays, objects) and refcount operations.** The milestone text's
   third named ingredient; has nowhere to attach until a reference-counted value exists in `ir::Ty`.
5. **Runtime-helper calls** — the milestone's fourth named ingredient, for `mixed`/union operands once they
   exist in the IR, and also what a non-`bool` `if`/`while` condition's ADR 0035 truthy conversion needs
   (this slice's arithmetic/conditions lower directly to native-shaped instructions with no helper
   fallback, since every operand type is a single scalar by construction).
6. `var` locals (ADR 0037) and full-magnitude/multi-base integer-literal cooking (hex/octal/binary) are
   smaller, independent gaps that can land whenever convenient.

Once control flow and calls both lower, M2's own *Verify* bullet ("IR snapshot tests; no program in the
corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven snapshot suite, not just
hand-written fixtures — at that point M2 as a whole should be closeable and M3 (baseline Cranelift backend,
`Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).
