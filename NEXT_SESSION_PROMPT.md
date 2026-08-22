# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session stood up the `mwl-ir` crate and lowered its first slice** — M2's last named deliverable,
"lowering to a CFG/SSA IR carrying explicit safepoints, refcount operations and runtime-helper calls, with
a stable per-statement/per-edge id" (the milestone text already committed to "CFG/**SSA**", so that part
was a mechanical follow-through, not a fresh decision):

- **`crates/mwl-ir/src/ids.rs`** — `StmtId`/`EdgeId`/`BlockId`/`ValueId` newtypes and an `IdGen` that hands
  them out in one deterministic pre-order lowering walk, scoped per function (not process-wide). `StmtId`/
  `EdgeId` each carry the source `Span` that produced them, recorded for later lookup — this is the stable
  id [ADR 0018](docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)'s coverage/
  branch probes need, reserved now per the milestone's own "cheap now, expensive to retrofit" framing.
- **`crates/mwl-ir/src/ty.rs`** — a small, flat, representation-level `Ty` (`Bool`/`Int`/`Uint`/`Float`/
  `Void` so far), deliberately **not** `mwl_types::ty::Ty`: the checker's type exists to reject the wrong
  program (qualifiers, nominal identity, unions); by the time a function reaches this crate it has already
  been proven to type-check, so the IR only needs to know how a value is *represented* for codegen. See the
  module's own docs for the full reasoning — this is the kind of split worth knowing about before adding to
  either enum.
- **`crates/mwl-ir/src/ir.rs`** — the data model: `Program` of `Function`s, each a `Vec<BasicBlock>` (this
  slice only ever produces one) of SSA `Inst`ructions ending in one `Terminator`. `Terminator::Branch`
  already carries the `EdgeId`-tagged pair of outgoing edges ADR 0018 will need, and `Terminator::Jump`
  exists too — neither is constructed yet, both reserved for when control flow lands.
- **`crates/mwl-ir/src/print.rs`** — a text pretty-printer (`fn add(int, int) -> int { bb0: v0 = param 0 ;
  int ... }`) used by snapshot tests today, intended for a future `mwl run --dump-ir`-shaped CLI flag once
  one exists (M3, alongside `--dump-asm`).
- **`crates/mwl-ir/src/lower.rs`** — `lower_method()`, the actual lowering, scoped to exactly one program
  shape: a method body of typed local declarations, plain `$x = expr;` reassignment, scalar unary/binary
  arithmetic and comparison operators over `bool`/`int`/`uint`/`float`, and `return`. No control flow at
  all (no phi nodes needed yet — a straight-line body has exactly one predecessor for every use, which is
  why this was the first slice). **Trusts its input already passed `mwl_types::check_program`** rather than
  re-checking it — panics, naming the unsupported shape, for anything outside scope. Five unit tests with
  `insta` snapshots cover straight-line arithmetic, reassignment producing a fresh SSA value (a bare
  `$y = $x;` needs no new instruction at all — SSA value-numbering falls out for free), unary/comparison
  operators, ADR 0007 § 4's uint-literal defaulting, and a `should_panic` proving control flow is refused.
- **Deliberately no dependency on `mwl-hir`/`mwl-types` yet.** Every type this slice's lowering needs
  (a parameter's, a local's, a method's return type) is read straight off the `mwl-syntax` AST, since ADR
  0007 § 1 already requires it spelled out there in full for every shape in scope — no name resolution or
  persisted checked-expression type is needed to answer "what type is this" *for this slice*. Widening past
  scalars will need one of: (a) `mwl-ir` depending on `mwl-hir`/`mwl-types` and re-deriving types itself
  (duplicating logic `mwl_types::expr` already has, currently all `pub(crate)`), or (b) `mwl-types` growing
  a published, persisted typed-expression table (e.g. keyed by `Span` or by a stable per-expression id)
  that lowering reads back rather than recomputes. **This is a real design decision for whoever picks up
  widening — it wasn't forced this session because straight-line scalar code never needed it, but property
  access/calls/`new` all will.** Flag it and decide deliberately rather than drifting into whichever shape
  the first widening PR happens to need.

`cargo build`/`test`/`clippy --all-targets -- -D warnings`/`fmt --check` all clean across the whole
workspace (mwl-types still at 154 tests, untouched; mwl-ir adds 5). Landed in three commits: crate
skeleton (ids/ty/ir/print), the lowering slice + snapshot tests, and the M2 status-block update.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. **Control flow.** `if`/`while` first (the two structures that need exactly one join point each), then
   `for`/`switch`/`try`. This is where SSA's real question — the join/phi-node algorithm — actually gets
   answered; the first slice deliberately dodged it. `Terminator::Branch`'s `EdgeId`s should get their
   first real construction here, and `crate::ir` may need a `Phi` instruction kind added.
2. **Safepoints.** Belong at a loop back-edge and at function entry (recursion) — neither exists until (1)
   lands loops. Reserve the instruction/marker shape when you get there; no guard test needs it until M3.
3. **Calls, `new`, property/array access.** This is where the "does `mwl-ir` depend on `mwl-types`, or does
   `mwl-types` publish a typed-expression table" decision above has to be made — resolve it before writing
   the lowering code, not after.
4. **Non-scalar values (`string`/`bytes`, arrays, objects) and refcount operations.** The milestone text's
   third named ingredient; has nowhere to attach until a reference-counted value exists in `ir::Ty`.
5. **Runtime-helper calls** — the milestone's fourth named ingredient, for `mixed`/union operands once they
   exist in the IR (this slice's arithmetic lowers directly to native-shaped `BinOp`/`UnOp` with no helper
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
