# Handoff

## State

**Goal `class-scoped-types` is reached.** All five stages are green, and the three gates a goal meets
only at its end are clean: `python tools/verify.py --doc` resolves every link, and `python
tools/owners.py --closes class-scoped-types` and `python tools/playbook.py --closes
class-scoped-types` each report the goal owns nothing.

**The floor check that held the previous run is green.** `python tools/owners.py` prints `of the 78, 0
owned by a retired goal`, with `retired-owner: 0`, `past-milestone: 0` and `unowned: 0`, and `python
tools/playbook.py --check` prints `none -- every carried-gaps owner is live or struck`. The failure
this session opened on was recorded against the tree as it stood before goal `decided-closures` took
the forty-eight items goal `unowned-closures` had tagged to itself — it is stale, not a regression.

**Next is goal `worker-placement`.** Its own `docs/agent/goals/62-worker-placement.handoff.md` is what
`goal-switch.py` installs; the group below is that file's, carried here so nothing is lost if the
switch is made by hand.

## Next group

**Stage 2: the seam a started core can install** — one file set:
`crates/nvs-runtime/src/script.rs`, `crates/nvs-host/src/placed.rs`, `crates/nvs-host/src/worker.rs`,
`crates/nvs-cli/src/main.rs`.

- [ ] **A resolver that can be published rather than borrowed** —
      `crates/nvs-runtime/src/script.rs:218`'s `install` takes a `&'static dyn Resolver` and `:241`'s
      `scoped` a borrow on the installing core's own stack, so neither reaches a thread `nvs-host`
      starts for itself. Add the form a placing core writes and a started core installs on its own
      thread, leaving `resolve`'s answer and `ResolveError` exactly as they are.
      `rule:security/isolate-shares-nothing` is what bounds what may be shared this way.
- [ ] **The started core installs it** — `crates/nvs-host/src/worker.rs:745`'s `destination` is where a
      core is chosen and a scheduler thread is started for the first placement; that start is where the
      published resolver goes in, beside the reactor the inbox poke reaches.
- [ ] **`crosses` stops asking which form the entry is** —
      `crates/nvs-host/src/placed.rs:107` is `entry.is_method() && ctx.class_table().is_some()`; the
      class-table half stays, because it is a fact about the context rather than about the entry.
      Rewrite that module's `# Known gaps` and `destination_for`'s second question
      (`crates/nvs-host/src/placed.rs:94`) as what then runs, and `crates/nvs-host/src/group.rs:89`'s
      restatement with them.

## Backlog

- Goal `decided-closures` holds the 48 module-doc gaps with `Decided:` sentences and sits in front of
  `gap-zero` — `docs/agent/goals/`, and `python tools/owners.py` lists them.
- `docs/agent/carried-gaps.md` carries 54 open rows; `playbook.py --check` says every owner is live.
- `docs/agent/playbook.md` is 1241 bullets and grows faster than its two staleness signals can prune —
  only the hand-fired pass in `docs/agent/doc-cleanup.md` shrinks it.
