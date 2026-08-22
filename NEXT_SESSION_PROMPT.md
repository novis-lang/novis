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

**Last session landed `mwl-ir`'s reserved safepoint markers** — item 2 in the widening order below, small
and self-contained as planned:

- **`crates/mwl-ir/src/ir.rs`** gained `InstKind::Safepoint`: a marker instruction, like `StmtMarker`,
  that defines no value and is inert — it lowers to nothing today. Its doc comment names the two fixed
  sites the project-start "safepoints from the first backend commit" decision calls for (function entry,
  a loop's back edge) and why the shape is reserved now rather than retrofitted once M3's codegen exists,
  mirroring `ids.rs`'s own "cheap now, expensive to retrofit" framing for `StmtId`/`EdgeId`.
- **`crates/mwl-ir/src/lower.rs`** emits one via a new `Lowering::emit_safepoint(&mut self, b: BlockId)`
  helper: once as the very first instruction in a function's entry block (`lower_method`), and once in
  `lower_while`, on the loop body's actual back edge — the last instruction appended to `body_cur` before
  it's sealed with `Terminator::Jump(header_block)`. Placed on the back edge itself, not the header, so a
  body that never reaches it (e.g. it always `return`s) polls zero times for that path, same as a
  functional poll would.
- **`crates/mwl-ir/src/print.rs`** renders it as a bare `safepoint` line, matching `StmtMarker`'s
  early-return shape in `print_inst`.
- All 8 existing snapshot tests were regenerated via `cargo insta test --accept -p mwl-ir` (every function
  now shows a leading `safepoint` line; `while_loop_carries_locals_through_a_header_phi`'s snapshot also
  shows one right before its `jump bb1` back edge). No new test was added — the existing control-flow
  snapshots already exercise both emission sites; a `for` loop is still out of scope so it gets no back-edge
  marker yet. `mwl-ir` stays at 9 tests; mwl-types stays at 154. `cargo build`/`test`/
  `clippy --all-targets -- -D warnings`/`fmt --check` all clean across the whole workspace.
- `docs/implementation-plan.md`'s M2 paragraph updated to describe this instead of the old "nothing marks
  it as a poll site yet" gap note.
- **Not done, and deliberately out of scope this slice:** the marker does nothing — no CPU-limit check, no
  cancellation check, no cycle-collector hook. That's M3's backend work, once Cranelift codegen exists to
  lower it to an actual poll. No guard test needs it functional before then.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.** `for`/`switch`/`try`, plus `break`/`continue` of any kind, are
   still out of scope — reuse `merge_envs`/the seed-then-patch phi dance rather than inventing a new
   algorithm; `switch`'s fallthrough-by-omitted-`break` shape and `try`/`catch`'s exceptional edges are the
   two likely to need something beyond a straight port. `for` will also need its own back-edge
   `emit_safepoint` call, mirroring `lower_while`'s.
2. ~~Safepoints.~~ **Done** (reserved shape only — see above). Revisit once M3's codegen exists to give the
   marker an actual lowering; no action needed in `mwl-ir` itself until then.
3. **Calls, `new`, property/array access — the next slice, and it starts with a decision only a dedicated
   session should make, not one made under time pressure mid-slice.** `mwl-ir` needs to know a call's or
   `new`'s resolved type (the callee's return type, the constructed class), which `mwl_types::expr` already
   computes but keeps `pub(crate)` rather than persisting anywhere `mwl-ir` can read it back from. Two ways
   to resolve this, unchanged from the last two sessions' carry-forward because nothing has forced the
   choice yet:
   - **(a) `mwl-ir` depends on `mwl-types`** and duplicates/reuses a chunk of `mwl_types::expr`'s
     inference logic (would need loosening some `pub(crate)` visibility, or a second inference pass).
   - **(b) `mwl-types` grows a new, deliberately-designed public typed-expression table** that
     `check_program` populates once and `mwl-ir` reads afterward, keeping the two crates' concerns separate
     but adding a new persisted data shape `mwl-types` has to maintain.
   This is a real architecture tradeoff (crate coupling and duplicate logic vs. a new public surface and
   the memory/maintenance cost of persisting a typed-expression table per compilation) — per CLAUDE.md's
   "ask about tradeoffs" rule, a session picking this up should present both options and their
   performance/memory/simplicity tradeoffs explicitly before writing any lowering code for calls, `new`, or
   property/array access, rather than picking one silently.
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
