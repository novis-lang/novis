# Handoff

## State

**The failing acceptance check is closed.**
`tests/conformance/reject/an-implementor-without-a-no-argument-constructor-is-named.nvst` is
written and passes. It pins ADR 0061 § 3's second refusal with both sides of the bound in one
refusing file: `App\Ambient`, whose constructor parameter is *defaulted*, sorts before
`App\Needy` in the name-ordered walk and is accepted silently, so the case fails a rule that
counted parameters rather than required ones. That third case also took
`Core\Program::implementing` off `BELOW_THE_FLOOR`.

**Item 11's `str.rs` and `time.rs` families are finished.** Eleven sites left `OWED_A_CASE`.
Nine are declarations, each probed with `nvs check`/`nvs run` before the comment was written:
`str.rs`'s `fromCodePoints` (`E0401` on `array<uint>`), `normalize` (`E0401` on
`Core\NormalForm`, probed with `mixed` and with the bare `int` literal), `format`'s argument
list (the `CoreTy::Variadic` judgement `Core\Path::join` states in full), the two
`usize`→`u64` counters, `replaceAll`'s key guard (an array key is `int|string` and a packed
key renders as its own digits, so ADR 0009 § 1 makes it valid UTF-8), `time.rs`'s three
`instant_of` slot guards, and `fromEpoch`/`Time::at`'s `uint` options.

**Two of the eleven were reachable, and were a bug rather than a boundary.**
`Core\Time\Date::format` and `Core\Time\TimeOfDay::format` placed their value on the timeline
before rendering, and `jiff`'s instant range is one day narrower than its civil range at each
end — so `Core\Time\Date::at(-9999, 1, 1)` and `::at(9999, 12, 31)`, both inside the year range
that member's own refusal names, aborted the request with an uncatchable `FATAL`.
`cldr::render` now splits into `render_placed` over a `civil::DateTime` plus an offset and a
zone name, with `render_utc` for a value that names no instant; both filters already refuse a
zonal field, so no pattern's bytes moved. Pinned by
`tests/conformance/core/a-date-at-either-end-of-its-year-range-still-formats.nvst`, which
asserts both accepted ends and the first refused year on each side.

**Untouched:** item 12's classification (`UNCLASSIFIED`, `crates/nvs-stdlib/src/registry.rs`,
still 118).

**Found, not fixed:** a `bytes` array key ICEs in `nvs-ir` rather than being diagnosed — the
playbook bullet under *Writing a test case* has the reproducer and names the pass that owes the
refusal. `catch (Core\Error $e)`'s panic, from an earlier session, is still open beside it.

**Orientation gap, fourteenth session running:** `[context] adrs` still does not carry
`0085 §§ 1-4`, and nothing in the pack names the stage-5 `[[check]]` blocks' `cases`/`tests`
lists — an acceptance failure naming a `.nvst` still has to be triaged by `grep`ping
`loop-goal.toml`, which is that list's only home.

## Next group

**Item 11's last big module, then item 12's next class. Shared file set:**
`crates/nvs-stdlib/tests/conformance_coverage.rs:604` — `OWED_A_CASE` is the worklist and a
slice is done when its lines are gone — plus one `crates/nvs-stdlib/src/<class>.rs` per slice.

- [ ] **`arr.rs`'s seven remaining stems, goal § item 11.** All argument or option guards, so
      the `E0401` shape `arr.rs:829`'s landed declaration states in full — but probe each,
      because the two `Core\Order` ones are the enum shape `str.rs`'s `normalize` turned out to
      be and the `average` one is a counter, not a guard. Sites: `arr.rs:1747` (`chunk`),
      `:2343` (`fill`), `:2367` (`fillKeys`), `:2836` and `:2846` (`sort`), `:3000`
      (`sortByKey`), `:4152` (`average`).
- [ ] **`uri.rs`, `hash.rs` and `bytes.rs`'s stems, goal § item 11.** `uri.rs:934`,
      `hash.rs:473`, `hash.rs:616`, `bytes.rs:1109`, `bytes.rs:1116`, `bytes.rs:1336` — four
      guards and two that need judging, `hash.rs:473` naming a `Core\Digest` case.
- [ ] **`Core\Validate` (6) and `Core\Uuid` (2), all `Qual::Neutral`, goal § item 12.** The
      rows are in `crates/nvs-stdlib/src/registry.rs`; ADR 0088 § 2 is the rule and the
      unclassified default is what refuses `tainted`.

## Backlog

- A `bytes` array key ICEs in `nvs-ir` (`lower/expr.rs:1972`) — owed a `nvs-types` diagnostic
  beside ADR 0007 § 5's float/bool/null refusal.
- `catch (Core\Error $e)` panics in `nvs-ir` instead of diagnosing — playbook, *Writing a test
  case*.
- `Core\Router::urlAbsolute` is at two cases against a floor of three
  (`conformance_coverage.rs`'s `BELOW_THE_FLOOR`).
- Item 12's 118 `UNCLASSIFIED` rows — `crates/nvs-stdlib/src/registry.rs`, ADR 0088 § 2.
- `[context] adrs` needs `0085 §§ 1-4`, and the pack needs the stage-5 `cases`/`tests` lists —
  `docs/agent/loop-goal.toml`.
