# Handoff

## State

**Stage 9's five acceptance checks are green.** `Core\Cldr::pluralCategory` is on disk, and
`plural_category_answers_from_the_carried_cldr_data` — the test the driver names — runs in
`crates/nvs-stdlib/src/cldr.rs`'s own `tests` module.

**The item's premise was false and the ADR carried it.** ADR 0082 § 2's row said the member
would expose "the CLDR data `nvs_stdlib::cldr` already holds"; that module held the § 4 date
pattern grammar and no plural data at all. So the slice was the member *and* the table: 19
`RuleSet` arms over ~170 language subtags in `RULES`, cardinal rules only. The ADR row is
amended to state what is now true; `cldr.rs`'s module doc is the home of every decision behind
it and none is restated here.

**A language outside the roster throws rather than falling back to English.** That is the one
call worth knowing without opening the module: it is ADR 0095's refusal, and the alternative is
silently wrong for exactly the languages the member exists for. The named absences and the
absent ordinal rules are gaps 3 and 4 in that module doc.

**`Core\Cldr` declares no capability**, for `Core\Storage`'s reason and a stronger one — the
answer is a function of two arguments and a table compiled into the binary, so ADR 0118 § 1 has
no door to check at.

**The pack printed everything this item needed.** No `[context]` gap found this session.

## Next group

**`Core\IO`'s two owed members — the plan's stage 2 line — over
`crates/nvs-stdlib/src/io.rs`, `crates/nvs-stdlib/src/registry.rs` and
`crates/nvs-stdlib/src/lib.rs`, plus `tests/conformance/io/`.**

- [ ] **`Core\IO\File::truncate`** — spec § 8's handle half, the instance roster at
      `crates/nvs-stdlib/src/io.rs:761` (the members run to `close` at
      `crates/nvs-stdlib/src/io.rs:822`), the `address` arm at
      `crates/nvs-stdlib/src/io.rs:1038`. It needs no new `CAPABILITIES` row: the handle was
      already opened under `fs.write`, which is the same door `Core\Storage` reuses. Read
      `nvs_runtime::capability` first for whether a truncate reaches it or the `File` directly.
- [ ] **`Core\IO\File::lock`** — the same anchors, `crates/nvs-stdlib/src/io.rs:761` and
      `crates/nvs-stdlib/src/io.rs:1038`, and the one decision in the pair: an advisory lock's
      release has to be tied to the handle's own close at `crates/nvs-stdlib/src/io.rs:822`
      rather than to a second member, or ADR 0063 R20 has one operation reachable two ways.
      Decide it and record it in `io.rs`'s module doc.
- [ ] **Three `.nvst` cases each**, under `tests/conformance/io/`, over the members added at
      `crates/nvs-stdlib/src/io.rs:761`. `every_core_class_has_a_conformance_floor_of_three`
      counts per class and `Core\IO\File` is already over the floor, so these are the *depth*
      shapes (a bound asserted on both sides, agreement across the handle's members) and not
      another row of the same shape. `python tools/gaps.py --errors` ranks them, and **budget a
      case per new `Fault::thrown`**: the coverage gate wants the message text in a case's
      expected output, so plan the refusal cases with the members rather than after them.

## Backlog

- `Core\Mail`'s TLS and `AUTH` — blocked on there being no TLS stack in `nvs-stdlib` at all;
  `crates/nvs-stdlib/src/mail.rs:55` is the gap note, and it is `Core\Http\Client`'s blocker too,
  so it is one `rustls` slice serving both (pre-authorized, ADR 0051 § 4).
- `Core\Storage::list` — needs a read-directory door in `nvs_runtime::capability`, which is a
  capability-surface decision of its own; `crates/nvs-stdlib/src/storage.rs`'s module doc owns it.
- `Core\Cli::displayWidth` — the plan's stage 3 line, `crates/nvs-stdlib/src/cli.rs`.
- Reading `[log] target` — the plan's stage 7 line.
- Stage 10's `differential` gate wants 250 passing and the tree has 210 — 40 oracle cases under
  `tests/differential/`, and the only check in this goal that is a volume of cases rather than a
  member.
- CLDR **ordinal** rules, if `Web\I18n` ever asks — `crates/nvs-stdlib/src/cldr.rs` gap 4.
