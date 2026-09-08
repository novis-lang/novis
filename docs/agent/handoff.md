# Handoff

## State

**Goal 19 — the binding site and the runnable fixture have both landed; what is left is prose and two
open design questions.** `examples/parses.nvs` runs under `nvs run --request examples/parses.nvsr` and
prints the stage-5 check's five lines, and the three conformance cases stage 4 names are on disk and
green. Full verify: 8 of 8, conformance 1623.

**Two things the goal names are still open, and neither is code the fixture needed.** Nothing binds a
`#[Query]` parameter, and a class-typed `#[Query]` cannot carry a default at all — `E0451` wants a
literal of the declared type — so goal prose stage 5's "a `#[Query]` with a default" is not writable
today and the fixture spells both `tag` readings by hand over the same `parse`. Stage 4's `cargo-named`
checks still name five `-p nvs-runtime` tests that do not exist, one of which
(`a_segment_parse_refuses_is_no_match_rather_than_a_matched_bad_value`) states the answer this goal's
§ *Standing decisions* settled the other way; it is a check name to amend, not work to do.

## Next group

**Stage 5: the reference pages — one file set: `docs/reference/lang/50-classes.md`,
`docs/reference/lang/90-attributes.md`, then `python tools/reference.py`.** `docs/novis.md` is
generated from those two and is never edited by hand.

- [ ] **The global interface roster counts five and lists six** — `docs/reference/lang/50-classes.md:659`
      opens "Five interfaces are declared by the compiler" over a table that now carries `Parses` as its
      third row. Fix the count, and say in that paragraph's own terms what `Parses` is for, on the
      precedent `rule:classes/comparable` set when the roster last grew.
- [ ] **A `Parses` section beside `Comparable`'s** — `docs/reference/lang/50-classes.md:1005` is
      `# Comparable`, and `Parses` owes the same shape: the one required member
      `parse(tainted string $s): static`, `tryParse` as a default body on the interface rather than a
      second member, and the binding sites that ask a class for it. `rule:expressions/try-parse`.
- [ ] **The two prose rosters still end at `Core\Uuid`** — `docs/reference/lang/90-attributes.md:307`
      (a route capture and a `#[Query]`) and `:509` (a command argument and an `#[Option]`) list the
      admitted types by name, and the last entry is now "a class implementing `Parses`" rather than one
      class written out. `rule:routing/a-query-parameter-is-declared-like-a-capture` and
      `rule:security/route-capture-is-laundered-by-its-type`.

## Backlog

- Stage 4's five `-p nvs-runtime` `cargo-named` checks name tests no crate hosts, and one contradicts
  the goal's own standing decision — amend `docs/agent/loop-goal.toml` and the goal source together.
- Nothing binds a `#[Query]` parameter to a declared one; `rule:routing/a-bad-query-value-is-a-400`'s
  query half has no implementation at all.
- A class-typed `#[Query]` can carry no default (`E0451` wants a literal of the declared type), which
  goal prose stage 5 assumes it can — a design question, not a slice.
- Stage 5's diagnostic corpus: a class standing in a capture without the interface, naming the
  interface as the fix.
- `nvs run --request` matching the inbound against the table would make `Core\Request::route()` answer
  in a leg — `crates/nvs-cli/src/main.rs:1651`, and the playbook bullet under *Running things*.
- Goal prose stage 5 also names a command argument at a `Parses` class; the fixture has none.
