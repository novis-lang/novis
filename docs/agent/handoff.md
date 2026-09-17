# Handoff

## State

**Goal `class-scoped-types` is complete — all five stages are landed and green.** Stage 5 put the
decision and the rule on disk: `docs/decisions/0190.md` creates `types/class-scoped-alias` and
modifies `types/type-alias`, `python tools/rules.py --render` rewrote the chapter and
`ground-rules.md`, and `--check` and `python tools/records.py --check` are both clean.

**The run is held on something this goal did not cause.** The floor carries
`no module-doc gap names a goal that walked without closing it`, and `python tools/owners.py`
answers `40 owned by a retired goal`: every one of them names goal `unowned-closures`, which was
reached at session 0003. 31 of the 40 carry the user's own `Decided:` sentence — `zip.rs` gap 1 is
`Decided: Yes: read the Zip64 extra fields and end-of-directory record`, and nothing reads them —
so the goal went green having *tagged* its work rather than built it. Goal `gap-zero`
(`docs/agent/goals/66-gap-zero.md:20`) says what this means in as many words: *"This goal builds
nothing. If its gate is red when the run arrives, a closure goal went green without closing an item,
and the item is named."* The gate is red one goal early, and 40 items are named.

`gap-zero` cannot absorb them and no live goal is a plausible owner, so the disposition is the
user's: a new chain goal before `gap-zero` that builds them to their own `Decided:` sentences, a
strike of the ones that are really stated bounds, or an M9+ re-tag. **Nothing else about the tree is
blocked**, and every check but this one passes.

## Next group

**Stage 1: the floor's retired owner** — one file set: `docs/agent/goals/`, and the 40 `# Known
gaps` blocks `python tools/owners.py` lists under *OWNERS THAT WENT GREEN WITHOUT CLOSING THE GAP*.

- [ ] **Take the user's disposition and put it in the chain** — `docs/agent/goals/60-unowned-closures.md:1`
      is the goal that walked, and `docs/agent/goals/66-gap-zero.md:20` is why nothing later can
      absorb its debt. If the answer is a goal, `python tools/chain.py --new <slug> --before 66` is
      what makes one; its spec is thin on purpose, because each gap's `Decided:` sentence is already
      its specification.
- [ ] **Re-owner the 40 tags in one patch** — `crates/nvs-stdlib/src/zip.rs:87` and
      `crates/nvs-syntax/src/casing.rs:71` are two of them; `grep -rn "owner: unowned-closures"
      crates/` is the whole list, and `python tools/splice.py --patch` takes all 40 files at once.
      A gap that is struck instead loses its whole numbered item, not just its tag.
- [ ] **Prove the floor** — `tools/owners.py:1`. The check wants `, 0 owned by a retired goal` in
      the summary line; nothing else in that output may move, so `unowned: 0` and `untagged: 0` are
      read in the same run.

## Backlog

- The 8 gaps deferred to a milestone the program has already passed (M1, M6, M7, M8) fail the same
  register's other report — `python tools/owners.py`, § *DEFERRED TO A MILESTONE …*.
- `docs/rules/types/type-alias.md` still describes only the file-scope form's `use`/FQN resolution
  in its third paragraph; the class-scoped site's resolution is in `rule:types/class-scoped-alias`.
- Goal `class-scoped-types`'s own `[context]` manifest never needed widening this session — the
  pack printed every rule, record section and anchor stage 5 used.
