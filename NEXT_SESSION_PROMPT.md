# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

## Do this first: finish the uncommitted, currently-broken ADR 0043 code follow-up

The working tree has substantial **uncommitted** changes to `crates/mwl-syntax/src/ast.rs`,
`crates/mwl-syntax/src/parser.rs`, `crates/mwl-syntax/src/casing.rs`, `crates/mwl-hir/src/hierarchy.rs`,
`crates/mwl-hir/src/members.rs`, `crates/mwl-hir/src/requires.rs`, `crates/mwl-hir/src/resolve.rs`,
`crates/mwl-hir/src/symbol.rs`, and `crates/mwl-diagnostics/src/lib.rs` — someone (a concurrent or prior
session) started [ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)'s
long-queued code follow-up (removing `trait`/class-body `use Trait, ...;`/`insteadof` entirely, replacing
them with default/private interface method bodies and `implements Iface by $field;` delegation). **Do not
discard this diff — it is real, careful, well-documented work**, not noise: `mwl-syntax`'s AST/parser side
looks essentially finished (`TraitDecl`/`UseTraitMember`/`TraitAdaptation` are gone, `ImplementsClause`
carries the new `by_field`, module docs are updated in ADR-citing style consistent with the rest of the
crate). **`mwl-hir` was not finished to match** — `cargo check -p mwl-hir` currently fails with 6 errors, all
in `crates/mwl-hir/src/hierarchy.rs` and `members.rs`, all references to AST shapes `mwl-syntax` no longer
has:

- `hierarchy.rs`: `pending.insteadof` (field removed from `PendingLinks`), `pending.decl_span` (removed),
  `links.traits` (removed from `ClassLinks`, twice) — this file's whole trait-use/`insteadof` resolution
  pass needs to go, replaced by default/private-method and `by`-delegation resolution per the ADR's own
  *Consequences*/*Verification* sections (the one home for the exact task list — don't re-derive it from
  scratch here).
- `members.rs:371`: matches `ClassMemberKind::UseTrait(_)`, a variant that no longer exists.

Once `mwl-hir` compiles again, work through the ADR's own verification list (new
`E_INTERFACE_MEMBER_CONFLICT`/`E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE`/`E_DELEGATE_TYPE_MISMATCH`
diagnostics, `by`-delegation type-matching) before running the full test suite and committing. `cargo test
--workspace` will not pass until this lands — treat the broken build as the reason to prioritize this over
any fresh `mwl-ir` slice below.

## Landed this session (docs only, no code): ADR 0046 — `#[...]` attributes

[ADR 0046](docs/adr/0046-attributes-shape-literal-metadata.md) decides MWL's answer to "PHP doc-comments as
config, but without a declared attribute class": `#[Name(...)]`/`#[{...}]` shape-literal metadata (reusing
ADR 0036's object literal/shape-type machinery), compile-time-constant-only field values, and retrieval via
a new, narrow `Core\Attributes::get<T>`/`::all<T>` — resolved structurally and entirely at compile time,
deliberately *not* part of `Core\Reflect`. No code changes yet: the `#[...]` grammar/checker work lands in
M4 (already in the plan's M4 paragraph and Verify bullet), `Core\Attributes` itself lands in M8 alongside
`Core\Reflect`/`Core\Ast` (also already in the plan). Also touched: ADR 0008/0010's deferred
`#[Memoize]`/`#[Flags]` bullets now point at this ADR as the mechanism they were waiting on (neither
attribute itself is decided), and ADR 0033 gained a fifth `secret`-refusing sink (an attribute payload
position) since only a compile-time constant can appear there and a `secret` class constant is one. Nothing
for the next session to *do* about this beyond awareness — M2 doesn't touch it.

## `mwl-ir`: still the main line of work once the above is unblocked

**Last landed: `break`/`continue` for `while` loops (level 1 only)**, via a `Lowering::LoopFrame` pushed
around `lower_while`'s body, recording one `(BlockId, Env)` edge per `break`/`continue` reached at any
nesting depth, folded into the header phi-patch (for `continue`) or a new `merge_envs` join at `after_block`
(for `break`, which previously always exited with exactly `header_env`). No new `Terminator`/`InstKind` was
needed. Seven new tests (95 → 102).

**Still explicitly out of scope:** `break N`/`continue N` for `N > 1` or a non-literal level (panics naming
`Lowering::loop_exit_level`); either keyword inside `for`/`switch` (neither lowers yet); a `break`/`continue`
outside any loop reaching this crate unrejected (a `mwl-types` gap, defensively caught here).

**Recommended next picks, once `mwl-hir` compiles again:**

- **`for` loops** — reuses `LoopFrame` verbatim; the natural next pick per the plan's own milestone text.
- **`switch`** — needs PHP's fallthrough-by-default case semantics decided, plus a `break`-exits-the-switch
  frame kind distinct from `LoopFrame` (not a drop-in reuse — scope it deliberately).
- **The `mixed` runtime type-tag representation** (`mwl-ir`'s own known-gaps list, item 5) — the biggest
  remaining unblock, real design work rather than a mechanical slice. Pre-authorized to design and proceed.
- **A `...spread`/`&value` array-literal element** (same list, item 4) — its own design each (array-merge
  semantics; a reference-value representation).

Once control flow, calls, and property/array access all lower, M2's *Verify* bullet ("IR snapshot tests; no
program in the corpus produces an `Unknown` type") is worth a real corpus-driven snapshot suite, not just
hand-written fixtures — at that point M2 should be closeable and M3 (baseline Cranelift backend, `Hello
World`) can start.

**Also still open, independent and low priority:** a `set`-hooked property is exempted from ADR 0022's
constructor check entirely rather than verified against the hook's body; the identical question applies to
whether a `lateinit` + hooked property should discharge on the hook's first commit (ADR 0038's own
*Revisiting* names this, deferred to `docs/spec/`).

## Housekeeping

`python .claude/brief.py`'s "WHERE THE PLAN STANDS" section has been hitting its 4000-byte budget and
truncating for several sessions (the M2 paragraph in `docs/implementation-plan.md` is the largest single
contributor) — this is exactly the signal `DOC_CLEANUP_PROMPT.md` describes as "overdue for a trim pass."
The user runs that pass manually; flagging it again since it's now a recurring truncation, not a one-off.
