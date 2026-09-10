# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — has finished `crates/nvs-stdlib/src/db/` and
`time.rs`.** 69 items still name nobody and every one of them is in `nvs-stdlib`;
`python tools/owners.py --check --reasons` stays green.

**`db/mod.rs`'s eight went four out of the gap block and four tagged.** The four that left owed
nothing: the five-driver completeness statement and the pooled reset are now
§ *Every driver reaches every member, and all five are pooled*, `stream`'s absent `chunk` is
§ *`stream` declares no `chunk`, and the portal is why*, and the delimiting quoter folded into the
§ that already refuses a delimiter. The four that stayed are two `unowned` — bounds for a pool no
`[db]` block describes, and `Db\DbError` sitting outside spec § 10's tree so a refusal carries no
`issues` — and two `gap-zero`, which names `stream`/`streamAs`/`serverVersion` and `queryAs<T>`'s
three refusals in its own stage 5.

**The renumbering is what the work was.** Twelve citations name a gap by position, and they moved with
it: `row.rs`, `registry.rs`, `pool.rs`, `execute.rs`, `stream.rs`, `queue.rs`, `carried-gaps.md` and
goal `gap-zero`'s two files. `carried-gaps.md` § *Owned* rows 50 and 54 are re-owned to `gap-zero`, and
the `{timeout?: Duration}` row is **struck**: `db/registry.rs`'s `STATEMENT_OPTIONS` declares the
option, so the gap it named is closed — goal `gap-zero`'s stage 0 item 2 was that finding and now
carries the rule rather than the instance.

**`time.rs` went four to one.** `Core\Month`, `sleep`'s parked task and the `Comparable`/`Stringable`
answer are sections; the one gap left is the per-call pattern compile, which is
`crates/nvs-types/src/intrinsics.rs`'s intrinsic-pass question one layer down, so it rides that
register bullet rather than a new one.

**The floor check `python tools/rules.py --check` was red on this file**, which cited
`rule:core-classes/derive-field-types`. The rule is `rule:core-classes/derive-field-list`.

**`python tools/verify.py`: 9 of 9 green.**

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-stdlib`'s module docs,
which hold every item that still names nobody. The owner kinds are the goal's § *Standing decisions*
and `python tools/owners.py --help`; `--untagged` is the worklist and `--check --untagged-is-an-error
--reasons` is the gate. Nothing else in the tree cites `json.rs`'s or `cldr.rs`'s gap numbers, so the
renumbering a moved item forces is cheap in both.

- [ ] **Tag `crates/nvs-stdlib/src/json.rs:100`'s eight items** — at
      `crates/nvs-stdlib/src/json.rs:100`, `:108`, `:129`, `:143`, `:148`, `:154`, `:158` and `:167`.
      Items 1, 6 and 8 read as decisions already — "neither is worth a whole document's re-scan", the
      dotted path as built, "not worth carrying until something measures it". Items 3, 4 and 5 are one
      question, `rule:core-classes/derive-generates-what-is-missing`'s: whether the derive machinery
      stays a descriptor read by native Rust or becomes emitted IR, which is what decides where a
      parameter default's constant and a hand-written `toJson` lookup live — one shared `unowned`
      reason, not three. Item 2 is `rule:core-classes/derive-field-list`'s roster against
      `CodecTy::Opaque` and is the same knot `crates/nvs-stdlib/src/db/mod.rs:291` gap 4 names. Item 7's
      bound is the native stack, so weigh goal `resource-ceilings` before `unowned`.
- [ ] **Tag `crates/nvs-stdlib/src/cldr.rs:148`'s four items** — at
      `crates/nvs-stdlib/src/cldr.rs:148`, `:155`, `:164` and `:174`. Gap 1 is
      `crates/nvs-stdlib/src/time.rs:102`'s question one layer down and the register bullet naming
      `crates/nvs-types/src/intrinsics.rs` already carries the reason. Gaps 2–4 are goal `gap-zero`'s
      stage 6 group 2 by name — the plural rosters, the ordinals, the eight pattern letters — and
      `docs/agent/carried-gaps.md:54`'s row for the same three still names retired owner
      `carried-gaps`, so it is re-owned in the same slice.
- [ ] **Tag `crates/nvs-stdlib/src/uuid.rs:86`'s three items** — at
      `crates/nvs-stdlib/src/uuid.rs:86`, `:89` and `:94`, then
      `crates/nvs-stdlib/src/xml.rs:122` and `crates/nvs-stdlib/src/zip.rs:80` if the file set still
      holds.

## Backlog

- `crates/nvs-stdlib/src/db/open.rs:568`'s `settings_text` doc says "until arm selection lands"; gap 1
  now says it has (`nvs_types::expr::args`' `select_arm`), so what wants re-deciding is its thrown-not-
  fatal argument.
- Ten `carried-gaps.md` § *Owned* rows still name a retired owner (`python tools/playbook.py --check`);
  goal `gap-zero`'s stage 0 item 1 owns the sweep.
- `crates/nvs-stdlib`'s remaining 69 untagged items, `python tools/owners.py --untagged`.
- `nvs-stdlib`'s gap blocks hold several "not worth it" items that are decisions in a gap list; the
  goal's § *Standing decisions* is the authority to move each one.
