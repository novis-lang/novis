# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — has finished `crates/nvs-stdlib/src/db/`,
`time.rs`, `json.rs`, `cldr.rs` and `uuid.rs`.** 50 items still name nobody and every one of them is
in `nvs-stdlib`; `python tools/owners.py --check --reasons` is green over the 81 that are tagged.

**`json.rs` went eight items to five.** The three that left owed nothing: the integer-overflow band
folded into the § that already refuses the literal, the dotted issue `path` is
§ *An issue's `path` is the wire key…* and `isValid`'s allocation is its own §. Of the five that
stayed, all are `unowned` under three reasons — the codec roster against `CodecTy::Opaque` (what a
`decimal` and an `Instant` *are* on the wire, which is `db/mod.rs` gap 4's knot at the other door),
the descriptor-versus-emitted-IR question that gaps 2, 3 and 4 all wait on, and the encoder's native
stack bound, which goal `resource-ceilings` names out of its own scope.

**`cldr.rs` kept all four items and its numbering.** Gaps 2, 3 and 4 are goal `gap-zero`'s stage 6
group 2 verbatim — the pattern letters, the plural roster, the ordinals — so they name it; gap 1 is
`time.rs`'s per-call pattern compile one layer down and rides the intrinsics bullet in
`carried-gaps.md`. The numbering could not move: `time.rs:104` and `:4002` cite cldr gap 1 by number.

**`uuid.rs` went three to one, and the one that stayed was stale.** It said the `bytes` round trip
waits on a `nvs_runtime::Tag::Bytes` variant that does not exist; the tag is live
(`crates/nvs-runtime/src/value.rs:307`), and `crate::random`'s gap 1 it pointed at is
`Core\Random\Seeded`. What is actually owed is a decision about widening spec § 11's second table,
which is what it now says. `==`-is-identity and `v7`'s absent intra-millisecond counter are sections.

**`python tools/verify.py`: 9 of 9 green.**

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-stdlib`'s module
docs, which hold all 50 items that still name nobody. Owner kinds are the goal's
§ *Standing decisions*; `--untagged` is the worklist and `--check --untagged-is-an-error --reasons`
is the gate. Goal `gap-zero`'s stages 5 and 6 name three of these file sets outright, so the owner
is evidence rather than a judgement — read them with
`python tools/peek.py docs/agent/goals/43-gap-zero.md:149-191`.

- [ ] **Tag `crates/nvs-stdlib/src/queue.rs:42`'s five items** — at
      `crates/nvs-stdlib/src/queue.rs:42`, `:45`, `:48`, `:56` and `:61`. Goal `gap-zero`'s stage 5
      is `Core\Db` **and `Core\Queue`**, so start from what that stage names and tag the rest;
      `rule:core-classes/queue-storage-is-a-table` is the rule the module implements, and item 5's
      two-dialect leg is a test-infrastructure fact rather than a gap — weigh moving it out.
- [ ] **Tag `crates/nvs-stdlib/src/registry.rs:23`, `crates/nvs-stdlib/src/process.rs:54` and
      `crates/nvs-stdlib/src/html.rs:59`** — goal
      `gap-zero`'s stage 6 groups 1 and 2 name all three by file and by subject (`Core\Metrics`'
      class row, `Core\Process::spawn`, `html-to-source`'s computed `$reason`), so each is
      `gap-zero`. One item each, three files, no renumbering.
- [ ] **Tag `crates/nvs-stdlib/src/lib.rs:77`'s four items** — at `crates/nvs-stdlib/src/lib.rs:77`,
      `:101`, `:170` and `:181`. These are the registry's own spec coverage (§§ 13–20) and the
      `array<T>` invariance one, so they decide how a whole-spec gap is owned; `:101` reads as a
      settled answer already ("Every shape a §§ 1–12 signature writes can now be stated").

## Backlog

- 50 gap items still name nobody, all in `nvs-stdlib` — `python tools/owners.py --untagged`.
- The gate's two flags are not in `verify.py` yet; goal `gap-owners`'s later stage owns that.
- `docs/agent/carried-gaps.md` § *Unowned* carries its entry count in prose (28) and nothing derives
  it — a bullet added without bumping it makes the file wrong.
- Other "waits on X" gaps may name a blocker that has landed, the way `uuid.rs`'s did; the check is
  one grep per blocker before the owner is chosen.
- `crates/nvs-stdlib/src/xml.rs`'s two cannot name goal `xml-tree`: that entry has no `.toml` and is
  retired, so they are `unowned` or a milestone's.
