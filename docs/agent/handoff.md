# Handoff

## State

**Goal `gap-owners`, stage 3 is green over everything the tool can see.** `python tools/owners.py
--check --untagged-is-an-error --reasons` reports 121 tagged gaps and no untagged item, and `python
tools/chain.py --check` walks 44 goals — all three of the stage's checks pass.

**The goal is not met, because the gate can go green over a module that names nobody.** Eight
modules record their gaps under a **bold** run rather than a heading, and `tools/owners.py:98`'s
`GAPS` regex is applied to headings only, so the roster has never seen one of those items. The
playbook bullet at `docs/agent/playbook.md:1471` already carried this; it is now the next group.

**The last seven untagged items resolved three ways.** `response.rs` gap 1 is M7's — `docs/plan/m7.md:39`
enforces the `echo` binding table from there, the HTML sink attached by a request and by nothing
else. Four are `unowned` with reasons: `random.rs` gap 1, `response.rs` gap 2, `test.rs` gaps 1–2.
Two left their blocks as decisions — `random.rs`'s fork gap, refused by M7's own `Type=notify` unit
and by `rule:core-classes/process-is-argv-only`, and `storage.rs`'s missing `exists`, whose own
first sentence called it deliberate.

**`storage.rs` cited two shape rules by the wrong number** — R7 and R20 where `rule:core-api/shape-rules`
puts "absence is `?T`" at R4 and "nothing is reachable two ways" at R17. Both are corrected.

**`carried-gaps.md` § *Unowned* is forty-nine entries**, and `python tools/verify.py` is green.

## Next group

**Stage 3: the attribution pass, the blocks the roster cannot see** — one file set: `tools/owners.py`
and the eight module docs holding a bold gap run. Owner kinds are the goal's § *Standing decisions*;
`python tools/owners.py --untagged` is the worklist and `--check --untagged-is-an-error --reasons`
the gate. Take the tool first: until it reads a bold run, none of the eight items can be tagged at
all.

- [ ] **Teach `owners.py` to read a `**Known gaps**` bold run as a block** — `tools/owners.py:98`'s
      `GAPS` regex, `tools/owners.py:142`'s block finder and `tools/owners.py:160`'s `items_of` are
      the three halves of it. `docs/agent/playbook.md:1471` is the trap and says the fix is the tool
      rather than a doc reshape; a bold run's items are `*`/`1.` bullets under one `//! **…**` line,
      so what changes is where a block starts and not how its items are cut.
- [ ] **Tag `crates/nvs-types`'s four bold runs** — `crates/nvs-types/src/ctor_init.rs:36`,
      `crates/nvs-types/src/lateinit.rs:34`, `crates/nvs-types/src/locals.rs:80` and
      `crates/nvs-types/src/signatures.rs:19`. Two of them say "deliberately out of scope for this
      slice", which under § *Standing decisions* reads toward a decision that moves above the
      heading rather than toward a tag.
- [ ] **Tag the remaining four** — `crates/nvs-hir/src/members.rs:68`,
      `crates/nvs-hir/src/requires.rs:69`, `crates/nvs-lsp/src/completion.rs:121` and
      `crates/nvs-syntax/src/casing.rs:57`. `requires.rs` holds five items, the largest of the eight.

## Backlog

- Whether a bold run should stay legal at all, or `owners.py` should refuse one, is the decision the
  first item above implies — `docs/agent/loop-goal.md` § *Standing decisions* is where it belongs.
- `crates/nvs-types/src/response.rs` does not exist under that name; `response.rs` gap 2 names
  `nvs_types::response` as E0801's home, and the module it means was not located this session.
- `python tools/decisions.py --check` stands at ~25 findings — a user-fired chore per
  `docs/agent/decisions-summary.md`, and no goal may gate on it.
