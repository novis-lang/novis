# Handoff

## State

**Goal `gap-owners`, stage 3 is green over every gap block in the tree.** `python tools/owners.py
--check --untagged-is-an-error --reasons` reports **147 tagged items in 69 blocks across 68 files**,
nothing untagged, nothing `unowned` without a reason, and `python tools/chain.py --check` walks 44
goals.

**The hole was closed in the tool, not by reshaping docs.** `tools/owners.py:141`'s `blocks` now
reads a `**Known gaps**` bold run as a block whose body ends at the next bold run or heading, and
whose first line is the label's own sentence. It was **21 blocks in 13 files, not the eight** the
previous scope named: the singular `**Known gap:**` spelling is thirteen of them.

**Ten statements left a gap block as decisions**, per the goal's § *Standing decisions*.
`crates/nvs-hir/src/members.rs`'s four are each closed in `nvs-types` — verified at
`crates/nvs-types/src/expr/calls.rs:1604`, `crates/nvs-types/src/expr/members.rs:1560` and
`crates/nvs-types/src/lower.rs:181` — so that block, `aliases.rs`'s and `nvs-types/src/check.rs`'s
are gone rather than tagged.

**Twelve § *Unowned* entries were written and the register is sixty-three.** Three gaps took live
owners instead: `crates/nvs-lsp/src/completion.rs` is goal `workspace-index`'s (its own prose is
"the completion arms the current rules already admit but nobody wrote"),
`crates/nvs-runtime/src/budget.rs` is goal `resource-ceilings`' own acceptance line, and
`crates/nvs-stdlib/src/heap.rs` is goal `gap-zero`'s, beside the six stdlib rows it already carries.
Nothing is blocked.

## Next group

**Stage 4: the gate names itself** — one file set: `tools/verify.py` and `tools/brief.py`. Both
checks are `stage = "4 wired"` and both are the driver's current red line; the stage's own comment
header is "it stays true", so each is a tool growing the flag its check calls.

- [ ] **`verify.py --list` prints the gate's steps in order and runs none of them** —
      `tools/verify.py:375`'s `steps_for(opts)` is already the ordered list the gate walks and
      `tools/verify.py:663`'s parser has no `--list` to print it. The check is
      `docs/agent/loop-goal.toml:6776`, which asserts the order rather than the count, so print one
      step per line in `steps_for`'s own order and let `-p`/`--fast` narrow it the way a run does.
- [ ] **`brief.py --where unowned` routes to the unowned register and names its count** —
      `tools/brief.py:453`'s table of the homes that are not rules is where a keyword lands when no
      chapter holds it, and `tools/brief.py:476`'s `WHERE_CAP` bounds one answer. The check is
      `docs/agent/loop-goal.toml:6786`; `carried-gaps.md` § *Unowned* is the home, and the count is
      `python tools/owners.py --json`'s rather than a number typed into `brief.py`.

## Backlog

- `docs/implementation-plan.md:44` and `:51` send a reader to a module doc's `# Known gaps`; a bold
  run is equally a gap block now — the plan owns that pointer.
- `tools/owners.py:223`'s `unowned_paths` keys a reason on any mention of the path, so a reason can
  be satisfied by an unrelated entry (playbook, § *Tooling*).
- `crates/nvs-ir/src/lower/convert.rs:963` records a `# Known gaps` heading in a `///` item doc,
  which `tools/owners.py:89`'s `SOURCES`/`DOC` pair never reads — a third spelling, unowned.
- Goal `unowned-sweep` stage 0 rewrites four module docs when goal `input-shapes` lands —
  `docs/agent/goals/31-unowned-sweep.md:21`.
