# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — is under way.** `python tools/owners.py`
reports 122 items naming nobody, down from 133, and every tag in the tree resolves to a goal, a
milestone or a reasoned `unowned`. Stage 4 is untouched: `verify.py` does not run the owners gate
yet, so the acceptance check is still its only reader.

**`crates/nvs-types/src/lib.rs` is done, and three of its five items were not gaps at all.** The
`inout` both-sides-agree obligation is `crates/nvs-types/src/expr/args.rs:1039`, named and spread
argument positioning is `args.rs:265`, and a read of a value-less class constant is `E0792` before
lowering, not the panic the item described (`crates/nvs-ir/src/lower/expr.rs:292`). Four of
`rule:types/narrowing`'s five spellings narrow today (`crates/nvs-types/src/locals.rs:29`), so what
was one stale item is now `is` alone, owned by goal `type-test`. The 75-line paragraph that headed
the block was a changelog of landed work, which `AGENTS.md` rule 6 forbids; it is a body section
above the heading now, pointing at the module that owns each rule rather than restating it.

**`crates/nvs-types/src/intrinsics.rs`'s six are all `unowned`, and the block already said why.**
Five of the six state the decision in their own text — which of two module docs is right, whether
the roster grows a restriction column, whether `nvs check` reads configuration. No live goal (the
chain's live entries are 30–44) closes any of them.

**[carried-gaps.md](carried-gaps.md) § *Unowned* is ten entries**, two of them new — one per file
tagged this session, each naming what has to be decided rather than what nobody got to.

## Next group

**Stage 3: the attribution pass, module by module** — one file set: the six small blocks left under
`crates/nvs-types/src/`, ten items between them. The kinds are the goal's § *Standing decisions*,
the three owner kinds are `python tools/owners.py --help`, and `--untagged` is the worklist. The
live goals a tag may name are `docs/agent/goals/*.toml` — 30 through 44, and nothing below 30.

- [ ] **Tag `crates/nvs-types/src/links.rs:46`'s two items.** Gap 1 (a named argument is not
      folded) already has its own entry in `docs/agent/carried-gaps.md` § *Unowned*, so it is
      `unowned` with the reason written; gap 2 (an enum-case capture has no closed set to check
      against) needs one. Check each against the code first — this session found three of five
      elsewhere already closed.
- [ ] **Tag `crates/nvs-types/src/derive.rs:44`'s three and `crates/nvs-types/src/layout.rs:38`'s
      two.** `derive.rs`'s gaps 2 and 3 name `fromRow` and `Core\Json::decodeAs`, which are
      `rule:core-classes/derive-attribute`'s and may belong to a live goal rather than to
      `unowned`.
- [ ] **Tag the three one-item blocks** — `crates/nvs-types/src/commands.rs:66`,
      `crates/nvs-types/src/error_lib.rs:47` and `crates/nvs-types/src/reasons.rs:44`.
      `reasons.rs`'s spread argument is the same cause as `links.rs`'s and `intrinsics.rs`'s, so it
      joins a reason that is already written rather than getting a third.

## Backlog

- `crates/nvs-types/src/expr/quals.rs:25` says § 5's auto-escape and `Markup + Markup` wait on
  `Core\Html` existing; both exist (`crates/nvs-types/src/expr_table.rs:734`). Stale prose, not a
  tagged gap.
- `crates/nvs-types/src/lib.rs:1` opens as a changelog — "M2's last open thread … this one adds" —
  which `AGENTS.md` rule 6 forbids. Left alone; it is not in the gap block.
- `docs/implementation-plan.md:104` names goals by number ("goals 4, 5 and 19"), which the slug
  rule forbids and `python tools/chain.py --check` does not catch in that file.
- `crates/nvs-types/src/lib.rs`'s `# Layout` list names eleven of the crate's modules and skips
  `intrinsics`, `links`, `reasons`, `commands`, `core_lib` and `error_lib`.
- Stage 4 is unstarted: `verify.py` still does not run `owners.py`
  (`docs/agent/loop-goal.toml`).
