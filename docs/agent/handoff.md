# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — has tagged 19 of the 26 items that named
nobody.** Seven are left, all in `crates/nvs-stdlib`: `random.rs` gaps 1–2, `response.rs` gaps 1–2,
`storage.rs` gap 1 and `test.rs` gaps 1–2. `python tools/owners.py --check --reasons` is green over
everything tagged, so the only red in the gate is `--untagged-is-an-error`.

**`carried-gaps.md` § *Unowned* went from thirty-seven entries to forty-five**, one per gap tagged
`unowned`: `debug.rs` gaps 1–2, `regex.rs` gaps 1 and 3, `cli.rs` gap 1, `command.rs` gap 1 and
`path.rs` gaps 1–2. Each names what has to be decided, per the goal's § *Standing decisions*.

**One item left its block as a decision**, `decimal.rs` gap 2: `rule:types/arithmetic`'s
`decimal ⊕ decimal` row already states the refusal at a scale over 28, so `divRound` refusing rather
than narrowing is that rule and not a hole. It is now a paragraph above `# Known gaps`.

**Two module-doc claims were corrected in passing, each with the evidence that makes them wrong now**:
`regex.rs` gap 2 no longer says there is no configuration subsystem (`crates/nvs-config/src/tree.rs`
is 1044 lines and simply names no key for the budget), and `debug.rs` gap 3 no longer says the crate
edge is missing (`crates/nvs-render/src/lib.rs:64` names `nvs-runtime` a dependent that already
renders through this crate).

**`python tools/verify.py`: green.**

## Next group

**Stage 3: the attribution pass, the last seven items** — one file set: `crates/nvs-stdlib`'s
remaining untagged module docs. Owner kinds are the goal's § *Standing decisions*;
`python tools/owners.py --untagged` is the worklist and `--check --untagged-is-an-error --reasons` is
the gate. The tag's shape is `//!    — owner: <goal slug|milestone|unowned>` as the item's **last**
line, and a `//! ` prose gap with no list indent takes it the same way
(`crates/nvs-stdlib/src/math.rs:42` is the shape). An `unowned` tag owes a bullet in
`docs/agent/carried-gaps.md` § *Unowned* naming the module path and gap number, and the section's
prose count — currently **forty-five** — is edited in the same pass.

- [ ] **Tag `crates/nvs-stdlib/src/random.rs:55`'s two items** — at
      `crates/nvs-stdlib/src/random.rs:55` and `:59`. Gap 2 waits on
      `rule:packaging/a-service-is-one-stored-argv`'s unbuilt `install`/`run`/`start`/`stop`/`status`,
      so its owner is whichever milestone's plan carries that registration — read the plan row before
      tagging, since `nvs service unit` already writes a file. Gap 1 is a whole class
      (`Core\Random\Seeded`) and reads like `M8`'s roster work, which
      `docs/plan/m4.md:56` is the precedent for: `Core\Cli` (§ 15) and `Core\IO` (§ 14) moved to M8.
- [ ] **Tag `crates/nvs-stdlib/src/response.rs:165`'s two items** — at
      `crates/nvs-stdlib/src/response.rs:165` and `:175`. Gap 1 is "the sink in force selects a
      rendering" and the module doc calls it the same gap as a request's HTML rendering seen from the
      other side, so decide the two together rather than separately. Gap 2 points at
      `nvs_types::response`'s own module doc for what `E0801` reaches — read that doc, because an item
      whose substance is another crate's may be a pointer rather than a gap.
- [ ] **Tag `crates/nvs-stdlib/src/storage.rs:83` and `crates/nvs-stdlib/src/test.rs:94`'s three
      items** — at `crates/nvs-stdlib/src/storage.rs:83`, `crates/nvs-stdlib/src/test.rs:94` and
      `:99`. Storage's item calls itself deliberate and cites `rule:core-api/shape-rules` R7 and R20,
      but that chapter's table gives R7 as *members are full words* and R20 as *no mutable/immutable
      twin types* — so either the citation is wrong and the real rule has to be found, or the item is
      an untaken decision and stays a gap. `test.rs` gap 1 says the refusal is `nvs_types`' to make.

## Backlog

- The other two stage-3 checks: `owners.py --check --reasons` (green today) and
  `python tools/chain.py --check` — `docs/agent/loop-goal.toml:6764` and `:6770`.
- `crates/nvs-stdlib/src/storage.rs`'s R7/R20 citation is a doc bug either way it resolves —
  `docs/rules/core-api/shape-rules.md:13` and `:26` are the rows it does not match.
- Whether a prose `# Known gap: <title>` section should carry its tag at list indent for consistency;
  `owners.py` accepts both today (`crates/nvs-stdlib/src/math.rs:42`).
