# Handoff

## State

**Conformance is at 547 of 600, and it is the only frontier left.** Verify is green (1597 cargo
tests, 74 suites, 547 conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt`
trees itself, so after a green `verify.py` there is nothing else to run (playbook, *Running
things*).

This session added no library code. It took the first two slices of the `Core\Time` group, both new
files under `tests/conformance/core/` and both agreement sweeps over `crates/mwl-stdlib/src/time.rs`:

- `time-date-members-undo-each-other-over-one-table.mwlt` — one ten-row table carrying both century
  rules and three month ends, over which `at`/`format` are inverses in both directions (20),
  `plus`/`minus` undo each other in `Day` and `Week` (20), and — the claim worth the case — a
  `Month`/`Quarter`/`Year` round trip returns **exactly when** the outward step kept its day of the
  month, so clamping is the only reason it ever fails (30 agreements). `with` is the identity for
  the empty bag, each field alone and all three (50); `compareTo` is the table's own order over all
  100 pairs and antisymmetric on each. Its tail names the clamp on both sides: in 2024 the 29th of
  January is the last day whose `+1 Month` survives the return trip and the 30th is the first that
  does not, both landing on 29 February.
- `time-of-day-wraps-at-midnight-and-orders-by-the-same-clock.mwlt` — 429 offsets every seventh
  minute from a day before 09:05 to a day after, each asserted against `(545 + n) mod 1440`
  computed in the case, with `minus(-n)` agreeing with `plus(n)` on every one; a full turn is the
  identity in three units over ten rows (30); `with` is the identity on the four parts `format`
  just read (20); and `compareTo` over all 100 pairs is the sign of the difference between the two
  seconds-of-day each value *renders*. Its tail is the wrap at both ends to the nanosecond, and the
  one pair whose order the seconds-of-day key cannot see.

`orient.py`'s pack was complete for this work; nothing was fetched outside it beyond `time.rs`'s
`Date`/`TimeOfDay` registration and member bodies, and `cldr.rs`'s fraction field.

**A by-hand pass over `docs/adr/` is still in flight and is not loop work.** 103 modified ADRs plus
`ground-rules.md` have been uncommitted for four sessions now. That is [doc-cleanup.md](doc-cleanup.md)'s
pass, which AGENTS.md says the user fires and the loop never does. **Do not stage it and do not
`git commit -a`**: stage your own paths, exactly as `session.py --wrap` already does.

## Next group

Three slices. The first finishes `Core\Time` on `crates/mwl-stdlib/src/time.rs`; the last two share
`crates/mwl-stdlib/src/uri.rs`, so the cheap two-slice session is **[2] and [3] together** once [1]
is ticked. `docs/spec/01-core-library.md` § 4 owns the `Time` rules and § 7 the `Uri` ones. Real
depth after this session: `Core\Time\Duration` 0.37, `Core\Uri` 0.53 — and read the new playbook
bullet before trusting `gaps.py`'s two thinner rows.

- [ ] **`Core\Time\Duration`'s constructors and its `to…` readers are one scale** — every unit
      constructor times its own factor is the same nanosecond count, `multipliedBy` and `negated`
      compose, and `parse` reads back what `toString` wrote for a table spanning both signs.
      `crates/mwl-stdlib/src/time.rs:473` (`parse`), `:545` (`multipliedBy`).
- [ ] **`Core\Uri`'s seven readers, `with` and `toString` are one parse** — over a table of absolute
      URIs each reader agrees with the slice `toString` renders, `with` handed the components
      `parse` just read is the identity, and `tryParse` answers `null` exactly where `parse` throws.
      `crates/mwl-stdlib/src/uri.rs:347` (`parse`), `:354` (`tryParse`), `:405`–`:454` (the
      readers), `:461` (`with`).
- [ ] **The four percent-coders are two inverse pairs over one byte sweep** — `encodeComponent`/
      `decodeComponent` and `encodeFormValue`/`decodeFormValue` each round-trip every ASCII byte and
      a multi-byte character, and the two encoders differ at exactly the bytes the form spelling
      reserves. `crates/mwl-stdlib/src/uri.rs:361`, `:368`, `:375`, `:382`.

## Backlog

- A `Core` enum in a user type annotation is `E0401` — `crates/mwl-types/src/core_lib.rs:292` interns
  the registry side; the source-written side resolves elsewhere. Playbook, *Writing MWL itself*.
- `gaps.py --coverage` under-reports a class whose values are never named — `tools/gaps.py:301`.
- `Core\ObjectMap` (0.78) and `Core\Validate` (0.83) are the next thinnest after `Core\Uri`.
- `Core\Json::decodeAs<T>` still reads a scalar-fielded class only — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
