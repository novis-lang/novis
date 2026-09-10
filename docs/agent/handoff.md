# Handoff

## State

**Goal `gap-owners` — a module doc's gap names its owner. Stage 2 is landed and green; stage 3 is
open and stage 4 is untouched.** `tools/owners.py` derives the roster from every `# Known gaps` block
in `crates/*/src/**`: 3 items tagged, 160 naming nobody. That count is the tool's to state.

**The `carried-refusals.md` question stage 3 opened is settled.** An entry there is a *reason*, never
an owner: a gap block whose sites it carries is `— owner: unowned` and points at the entry, and
`owners.py`'s `unowned_paths` now reads that file beside `carried-gaps.md` § *Unowned*. The goal's
§ *Standing decisions* carries the argument, and entry 901's struck owner is the evidence — it named
goal `typed-callable`, which retired with all fifteen sites open.

**The tree is shared right now.** `docs/agent/commands.md`, `tools/orient.py`, `tools/chain.py` and
`docs/agent/goals/42-finish-response.md` are the user's uncommitted edits, not a session's; stage
them for nothing. `chain.py --check` failed this session on `docs/zz-untracked-probe.md`, an
untracked probe of theirs, and passes now that it is gone.

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-ir/src/lib.rs`,
`crates/nvs-runtime/src/lib.rs`, `crates/nvs-stdlib/src/router.rs`, `docs/agent/carried-gaps.md`.

- [ ] **Tag `crates/nvs-ir/src/lib.rs:185`'s seventeen items** — the largest block, and the reading it
      waited on is landed. Seven are M4 lowering refusals entry 901 carries (gaps 1, 3, 4, 6 and the
      `for`/`switch` halves of 1) and take `— owner: unowned` pointing there. Ten do not, and each
      needs its own kind per the goal's § *Standing decisions*: gaps 5, 15 and 16 name panics
      `holes.py --item 901` does not list, gaps 9, 14, 18 and 21 say in their own text that the hole
      belongs to another crate's module doc, and gaps 2, 7, 11 and 17 are a leak, a lookup cost, a
      constant-time hole and an unobservable one. Resolve each toward a milestone tag or a live goal
      before reaching for `unowned`; `unowned` needs a `carried-gaps.md` § *Unowned* bullet naming the
      file, and the entry-901 reason does not stretch to a gap it does not carry.
- [ ] **`crates/nvs-runtime/src/lib.rs:188`'s seven items**, five of which the docs already decide.
- [ ] **`crates/nvs-stdlib/src/router.rs:40`'s gap 3 is closed or tagged, not both.**

## Backlog

- Stage 4: the full gate into `tools/verify.py`, and `brief.py --where unowned`; the goal's
  `4 wired` checks are at `docs/agent/loop-goal.toml:6774`.
- `[context] modules` names six paths that are not crate modules (`tools/*.py`,
  `docs/agent/carried-gaps.md`), and `orient.py` warns once per session for each; the field only
  matches a `//!` module doc.
- `carried-gaps.md` § *Owned* names seven retired goals; `owners.py` buckets a retired owner as a
  finding rather than a failure, the same way `tools/playbook.py --check` already does.
- The goal md's opening count (50 blocks, 152 items) counts a different thing from the tool, which
  also counts `- ` bullets and one-gap prose blocks. Left alone; `owners.py` is the home now.
- `unowned_paths` is file-granular in both files it now reads, so a second `unowned` item in an
  already-named file passes `--reasons` without a reason of its own.
