# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — is under way.** `python tools/owners.py`
reports 28 tagged gaps, every one resolving to a goal, a milestone or a reasoned `unowned`, and 133
still naming nobody. Stage 4 is untouched: `verify.py` does not run the owners gate yet, so the
acceptance check is the only thing reading it.

**The three largest blocks are done** — `crates/nvs-ir/src/lib.rs` (17 items),
`crates/nvs-runtime/src/lib.rs` (6) and `crates/nvs-stdlib/src/router.rs` (2). `nvs-ir`'s residue is
legitimately `unowned` and that is the finding, not a shortcut: M4 is carried, its acceptance list
passes, and no live entry on the chain writes `nvs-ir` lowering again — the argument
[carried-refusals.md](carried-refusals.md)'s item 901 makes for its refusal sites is the whole
file's. One item took a milestone instead: gap 7's by-slot vtable is a lookup cost, and M12's plan
carries inline caches for method access.

**Two items left their blocks entirely**, per the goal's § *Standing decisions* — a settled
non-goal in a `# Known gaps` list is a decision, and it moves out. `nvs-runtime`'s old gap 3 (the
request arena is refused, not missing) and `router.rs`'s old gap 3 (the verb parse is
`crate::request`'s) are body paragraphs now. A gap number is a stable identifier, so both holes
stay rather than renumbering what follows.

**[carried-gaps.md](carried-gaps.md) § *Unowned* is eight entries**, three of them new: one per
file tagged this session, each naming what has to be decided rather than what nobody got to.

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-types/src/`, whose
21 items are the largest untagged crate left. The kinds are the goal's § *Standing decisions*, the
three owner kinds are `python tools/owners.py --help`, and `--untagged` is the worklist.

- [ ] **Tag `crates/nvs-types/src/lib.rs:98`'s five items.** Gap 1 is a heading over the three ADRs
      rather than a shape, so it is the first candidate for moving out of the block the way this
      session moved two; gaps 2–5 (exhaustive reachability, one of four narrowing spellings, `inout
      $x` needing both sides declared, a promoted property's and a named argument's propagation) are
      checker holes M4 carried and no live goal reopens — resolve each toward a milestone or a goal
      before reaching for `unowned`.
- [ ] **Tag `crates/nvs-types/src/intrinsics.rs:38`'s six items.** Gaps 5 and 6 name
      `rule:core-classes/db-literal-query-checking` themselves, so the rule's own chapter is where
      their owner is argued; gap 4 (a named or spread argument is not read) is the same fold as
      `links.rs`'s gap 1 below and should not get a second reason.
- [ ] **Tag the six small blocks** — `crates/nvs-types/src/links.rs:46`,
      `crates/nvs-types/src/derive.rs:44`, `crates/nvs-types/src/layout.rs:38`,
      `crates/nvs-types/src/commands.rs:66`, `crates/nvs-types/src/error_lib.rs:47` and
      `crates/nvs-types/src/reasons.rs:44`. `links.rs`'s gap 1 already has its reason written:
      `carried-gaps.md` § *Unowned*'s new route-link bullet names that file, so `unowned` resolves
      there without a second bullet.

## Backlog

- `orient.py` warns every session that five `[context] modules` patterns name tools and a doc, which
  can never match a module doc — the driver sweeps that field from commit paths.
  `docs/agent/loop-goal.toml`.
- Stage 4: wire `owners.py --check --untagged-is-an-error --reasons` into `tools/verify.py`, which
  that tool's own help already names as where it runs.
- `crates/nvs-ir/src/ty.rs:16`'s one-item block is the leftover of this session's file set.
- 133 items still name nobody, across `nvs-cli`, `nvs-db`, `nvs-hir`, `nvs-server` and the rest of
  `nvs-stdlib`.
