# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed `break`/`continue` for `while` loops — the twenty-first `mwl-ir` slice, level 1
only.** A new `Lowering::LoopFrame` — pushed onto a new `Lowering::loop_stack` before `lower_while` lowers
its body, popped back off once it returns — carries the loop's `header_block`/`after_block` plus one
`(BlockId, Env)` pair per `break`/`continue` actually lowered inside that body (at any nesting depth
reachable through `if`/nested `{}`). `Lowering::lower_break`/`Lowering::lower_continue` just read the top
frame, record their own edge into it, and seal the current block with a plain `Terminator::Jump` to
`after_block`/`header_block` respectively — no new `Terminator`/`InstKind` shape was needed, confirming the
crate's own known-gaps prediction that `if`/`while` had already exercised everything `for`/`switch`/
`break`/`continue` would need.

The actual design work was folding those recorded edges into the two join points `lower_while` already
built: a `continue` is exactly one more loop back edge, so its edge joins the body's own fall-through exit
(when the body reaches one) before the existing header-phi-patch loop runs, widened from "patch with the
one fall-through value" to "patch with every back edge's value, fall-through included." A `break` is a new
kind of join `lower_while` never had before at all — before this slice, a loop's exit environment was
always exactly `header_env`, since the condition's own false edge was the only way out — so `after_block`
now runs the same `Lowering::merge_envs` general-purpose join `lower_if` already uses for its own merge
block, combining the false edge (carrying `header_env`) with every recorded `break` edge; with no `break` at
all this degenerates back to exactly the prior single-clone behavior (`merge_envs`'s own `[(_, only)]`
case), so a break-free loop's lowering is bit-for-bit unchanged apart from one extra clone.

**What's still explicitly out of scope, deliberately, from this slice:**

- `break N`/`continue N` for any `N > 1` (a multi-level exit) and a non-literal level expression both still
  panic naming the gap (`Lowering::loop_exit_level`) — unwinding more than one loop would need every
  `LoopFrame` up to the `N`-th on `loop_stack`, not just the innermost one, walked and folded in too.
- Either keyword inside a `for`/`switch` body — neither statement lowers at all yet.
- A `break`/`continue` with no enclosing loop at all still reaches this crate unrejected by `mwl_types`
  (it doesn't check loop nesting itself — a `mwl-types` gap, not an `mwl-ir` one) — `Lowering::loop_stack`
  being empty is this crate's own defensive check for that case, not something a well-formed input program
  could actually trigger.

Seven new tests landed (95 → 102): a plain `break`, a `break` that forces a genuine after-block phi (a
value differs between the loop's steady-state phi and the value live at the point of the break), a
`continue` that adds a third incoming edge to a header phi (alongside the pre-loop edge and the
fall-through back edge), `break`/`continue` outside any loop (should-panic), and a multi-level `break 2`/
`continue 2` (should-panic). Built, tested, clippy- and fmt-clean (scoped to `mwl-ir`; see the fmt note
below), committed. `crates/mwl-ir/src/lib.rs`'s module docs and `docs/implementation-plan.md`'s M2
paragraph were both updated in place with the new slice.

**`for`/`switch` themselves are still not lowered at all** — picking either up next is expected to reuse
the same `Terminator::Branch`/`ids::EdgeId`/`merge_envs`/phi-patch shapes `if`/`while`/this session's
`break`/`continue` all already exercise, per `lower`'s own module docs. `for`'s own `break`/`continue`
would need the exact `LoopFrame` machinery this session built, reused rather than re-invented — pushing a
frame around a `for` loop's own body lowering the same way `lower_while` now does. `switch`'s `break`
(PHP's normal, expected way to end a `case`) is a different shape again: it exits a `switch`, not a loop, so
it needs its own frame kind (or a widened one) rather than reusing `LoopFrame` verbatim — worth scoping
deliberately when that session starts, not assumed to be a drop-in reuse.

Recommended options for next session, roughly in order of how self-contained they are:

- **`for` loops** (without their own `break`/`continue` yet, or with it — your call once you're in it) —
  the next natural pick per the plan's own "M2, and the one after it" section, and the shape this session's
  `LoopFrame` was explicitly built to be reusable for.
- **`switch`** — needs its own case-fallthrough/`default` semantics decided (PHP's `switch` falls through by
  default, unlike `match`), on top of a `break`-exits-the-switch frame kind distinct from `LoopFrame`.
- **The `mixed` runtime type-tag representation** (see `mwl-ir`'s own known-gaps list, item 5) — the single
  biggest unblock left on that list, but real, non-mechanical design work rather than a narrow mechanical
  slice. You're pre-authorized to design and proceed if you pick this up.
- **A `...spread`/`&value` array-literal element** (item 4 in that same list) — each needs its own design
  (array-merge semantics; a reference-value representation), bigger and less mechanical than this session's
  pick was.

Once control flow, calls, and property/array access all lower (which is very close now — `for`/`switch`
are the last statement-level gaps), M2's own *Verify* bullet ("IR snapshot tests; no program in the corpus
produces an `Unknown` type") is worth revisiting for a real corpus-driven snapshot suite, not just
hand-written fixtures — at that point M2 as a whole should be closeable and M3 (baseline Cranelift backend,
`Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).

**Queued, independent of the `mwl-ir` work above: ADR 0043's code follow-up.** [ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)
(docs-only, landed several sessions ago) removes `trait`, class-body `use Trait, ...;`, and `insteadof`
from the language entirely, replaced by an `interface` method with a `public`/`private` body and one
`implements` entry carrying a `by $field;` delegation suffix. The ADR's own *Consequences* and
*Verification* sections are the one home for the exact task list — don't re-derive it here — but the shape
is: remove `mwl-syntax`'s `TraitDecl`/`UseTraitMember`/adaptation AST and grammar (replaced by a parse-time
`E_TRAIT_NOT_SUPPORTED` diagnostic) and add default/private interface-method-body grammar plus `by $field`
grammar; remove `mwl-hir`'s entire trait-use/`insteadof` resolution machinery (`hierarchy.rs`) and add
default/private method resolution, `by`-delegation resolution, and the new
`E_INTERFACE_MEMBER_CONFLICT`/`E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE`/`E_DELEGATE_TYPE_MISMATCH`
diagnostics. It's a large, separable chunk — check `mwl-syntax`/`mwl-hir` first to confirm it hasn't
already landed, then either fold it into a session alongside the `mwl-ir` picks above or, better, give it
its own dedicated session given its size. `mwl-hir`'s trait-flattening code is stale (still matches the
pre-ADR-0043 design) until this lands. This note has been carried forward for several sessions now without
anyone picking it up — if the `mwl-ir` picks above keep winning out, consider giving this one priority next
time instead, purely so it doesn't stall indefinitely.

**ADR 0045 (and/or/xor keyword operators rejected) has landed** — it was in-progress by a concurrent
session as of a few sessions ago and is now committed (`f2ce437` and prior). No action needed.

**Known pre-existing `cargo fmt --check` failure, unrelated to any `mwl-ir` work:** `crates/mwl-syntax/src/parser.rs`
(around the ADR 0045 diagnostic's `help` text and one of its own tests) fails `cargo fmt --check` on `main`
as of `f2ce437` — confirmed via `git stash`/`cargo fmt --check`/`git stash pop` that this predates and is
independent of this session's `mwl-ir` changes. Whoever picks up `mwl-syntax` next should run
`cargo fmt -p mwl-syntax` to fix it (a two-line formatting fix, not a design question) rather than being
surprised by a dirty `cargo fmt --check` on an unrelated change. Scope your own session's fmt check to the
crate you're touching (`cargo fmt --check -p <crate>`) until this is cleaned up, the same way this session
did.

**Housekeeping note:** `python .claude/brief.py`'s "WHERE THE PLAN STANDS" section has been hitting its
4000-byte budget and truncating for several sessions now (the M2 paragraph in `docs/implementation-plan.md`
is the largest single contributor) — this is exactly the signal `DOC_CLEANUP_PROMPT.md` describes as
"overdue for a trim pass." The user runs that pass manually; flagging it again here since it's now a
recurring truncation, not a one-off.
