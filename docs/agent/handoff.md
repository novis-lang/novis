# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0 and 2–10 are done**, and stage 11 (CLDR) has two of its three
items landed: every pattern letter the grammar once refused now formats, so
`crates/nvs-stdlib/src/cldr.rs`'s `# Known gaps` 2 is gone and the ordinal-roster gap has taken
its number — the stage's one remaining item is `# Known gaps` **2** now, not 3. Nothing is blocked.

The subset is closed over the ASCII alphabet again at a wider boundary: 37 letters are fields and
the 15 still refused are the ones CLDR reserves and gives no meaning, plus `V` at a count other
than `VV`. `U`, `r` and `q` resolve onto the letters they agree with under the Gregorian calendar
rather than carrying variants that render the same bytes; `z`, `v`, `O` and the long counts of `Z`
render the localized GMT format, which is ICU's own fallback where no zone name is transcribed —
and none is here.

**Two decisions are recorded in the module doc rather than in a record.** Day periods (`b`, `B`)
read English's, because § *No locale, by decision* already fixed the one locale every name in this
module renders in. And `Y`, `e`, `w`, `W` and `c` read **one** week rule rather than a transcribed
`weekData` table: nothing selects a territory, so a table would be read at one row — and that row,
CLDR's `001`, calls a short leading week week 1, which is not the rule `w` has always rendered.
Carrying both would put `w` and `Y` on different calendars inside one pattern.

## Next group

**Stage 11: CLDR, its last item** — one file set: `crates/nvs-stdlib/src/cldr.rs`, the ordinal half
of the module, which shares no code with the pattern compiler the two landed items changed.

- [ ] **The ordinal roster is CLDR's** — `crates/nvs-stdlib/src/cldr.rs:2585` is `ORDINALS`, the
      table of languages that mark an ordinal form, `crates/nvs-stdlib/src/cldr.rs:2384` is
      `OrdinalSet`, the shapes those rules take, and `crates/nvs-stdlib/src/cldr.rs:212` is gap 2,
      which states the failure mode: a language CLDR publishes a rule for but this table omits
      answers `Other` silently, where the cardinal roster throws. Widening it is a row per language
      and an arm only where the published rule is a shape no arm has;
      `crates/nvs-stdlib/src/cldr.rs:2645`'s `rules_for` is the boundary both members share and
      stays the refusal for a language neither table carries.
      `crates/nvs-stdlib/src/cldr.rs:3638`'s
      `an_ordinal_category_is_answered_for_every_language_with_a_published_table` is the test the
      new one sits beside. Closes
      `every_language_cldr_gives_an_ordinal_rule_is_on_the_ordinal_roster`, and with it stage 11.

## Backlog

- Stage 12 is `Core\Metrics`: three `nvs-stdlib` tests, two `nvs-types` ones and
  `tests/conformance/core/metrics-increment-observe-and-gauge-accumulate.nvst`
  (`docs/agent/loop-goal.toml:11060`) — a different file set, so a new group.
- `crates/nvs-stdlib/src/cldr.rs` gap 1, the compile-time prepared pattern, is `unowned-closures`'.
- A pattern's offset is written to the minute wherever an offset appears, including the localized
  GMT format, so a zone with a second-level historical offset renders truncated — `X` and `x` have
  always done this and the new letters agree with them.
