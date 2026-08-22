# Next session prompt

## State

Milestone **M2** (HIR/types/IR) is close to done; **M3** (baseline Cranelift backend → `Hello, World!`) is
the loop's target, per `.claude/loop-goal.md`. Run `python .claude/brief.py` first, then read
`docs/implementation-plan.md`'s M2/M3 paragraphs — the plan is the one home for status detail, this file
only points.

On disk: `mwl-diagnostics`, `mwl-syntax`, `mwl-hir`, `mwl-types`, `mwl-ir`, `mwl-cli` (`ast`, `check`),
`fuzz/`, `benches/abi-probe`. Workspace is green (build/test/clippy/fmt). `mwl-codegen` and `mwl-runtime`
do not exist yet — create them at M3 with their **own** `[lints]` block (`unsafe_code = "deny"`, narrow
reasoned allows), never `lints.workspace = true`, which is `forbid`.

Nothing is blocked. Three gaps sit directly on the acceptance command's path and are spelled out, with
their already-decided designs, in `.claude/loop-goal.md` § *The three gaps that actually sit on the path* —
read that section before picking work.

## Next

**The script body is a function** — the largest of those three gaps, and the one everything else waits on.
`mwl-types::check::check_stmts` walks declarations only (`_ => {}` swallows every top-level statement) and
`mwl-ir` exposes only `lower_method`, so a top-level `echo "...";` is today neither type-checked nor
lowered. Confirmed: `echo $undefinedThing;` at file scope passes `mwl check` clean, while the identical
line inside a method reports `E0301`. [ADR 0008](docs/adr/0008-static-and-global.md) § 2 already decides
the shape — a file's top-level statements are one synthesized frame whose variables are locals — so this is
reuse of `check_method`/`lower_method`, not a second walk. Land the checker half first; it is testable on
its own with the fixtures `check.rs` already has.

## Backlog

- `echo` lowering: no `StmtKind::Echo` arm in `mwl-ir`'s `lower.rs`, no `InstKind`/`Helper` behind it —
  `.claude/loop-goal.md` fixes the CLI semantics (raw stdout, no escaping).
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT` —
  `ImplementsClause.by_field` has parsed since M1 and is still unread by `mwl-hir`/`mwl-types`
  ([ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md) §§ 4-5).
- `for` loops in `mwl-ir` (reuses `LoopFrame` verbatim), then `switch` (needs a distinct frame kind and
  PHP fallthrough semantics decided) — `mwl-ir`'s own module docs hold the known-gap list.
- The `mixed` runtime type-tag representation — `mwl-ir` known-gap item 5, real design work, pre-authorized.
- A `set`-hooked property is exempted from ADR 0022's constructor check rather than verified against the
  hook body; same question for `lateinit` + hook ([ADR 0038](docs/adr/0038-lateinit-property-modifier.md)
  *Revisiting*). Low priority.
- `python .claude/brief.py`'s "WHERE THE PLAN STANDS" section is over its 4000-byte budget and truncating
  mid-sentence — every session gets a cut-off brief. `DOC_CLEANUP_PROMPT.md`'s trim pass is overdue; the
  user runs it manually.
