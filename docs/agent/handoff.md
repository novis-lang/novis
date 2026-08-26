# Handoff

## State

**Conformance is the only frontier left, at 494 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **494 passed, 0 failed** and `mwl test tests/` is **645 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than
it (playbook, *Running things*).

**`python tools/gaps.py --errors` is the worklist and it now prints 72 sites, of which 59 are
`fatal` and unreachable by any case — but read the assertable remainder as 14, not the 13 it
shows.** This session closed `Core\Path::withExtension`'s four and `Core\Time`'s three `format`
refusals in one new case each; the list dropped by twelve, because a stem match also hid
`time.rs:1968` (a real, unasserted `thrown`) and four `fatal` siblings. The new playbook bullet
under *Tooling* owns why, and the plan's `Open now` says what the two cases assert past their
rows. `path.rs` now holds no assertable site at all.

**`time.rs` is the whole of the next group**, and its four are `fromIso`, `Zone::of`, and
`Core\Time::parse`'s two `thrown_as` — the last of which are `mwl_stdlib::cldr::read`'s
sentences, not `time.rs`'s own, so the messages to assert are in `cldr.rs:487-680`.

**`orient.py`'s `[context] modules` needs `time.rs` and `cldr.rs`**, neither of which printed and
both of which every slice below reads. `path.rs` is already in it; `str.rs` and `encoding.rs` can
come out.

## Next group

Each slice reads `crates/mwl-stdlib/src/time.rs` (and, for the parse half, `cldr.rs`) for its
messages and writes **one** new file under `tests/conformance/core/` — that pair of modules plus
that directory is the file set all three share, and the modules are read-only here, so a second
slice costs one `peek.py` of its anchors. Every one is the *Edges* shape over a `Fault::thrown`,
closed with a counted claim and with the bound named on both sides where there is one.

- [ ] **`Core\Time`'s two unknown-zone refusals** — `crates/mwl-stdlib/src/time.rs:1844`
      (`Core\Time\Zone::of(): unknown time zone `{id}``) and `crates/mwl-stdlib/src/time.rs:1968`
      (the same sentence from whichever `DateTime` member was called, which is the `{member}`
      hole that hides it from `--errors`). Spec § 4 owns the rows. `Zone::UTC` and a real IANA
      identifier are the accepted side; the counted claim is that every member reaching a zone
      answers with **its own** name in front of one shared sentence.
- [ ] **`Core\Time::parse`'s two `thrown_as`** — `crates/mwl-stdlib/src/time.rs:2464` and `:2469`,
      whose `{why}` is one of `mwl_stdlib::cldr::read`'s five: a literal that does not match at an
      offset (`cldr.rs:490`), too few digits of a named field (`cldr.rs:677`), `expected AM or PM`
      (`cldr.rs:612`), trailing text with a character count (`cldr.rs:507`), and a zonal pattern
      refused because the zone is `parse`'s third argument (`cldr.rs:496`). The offsets are the
      bound to name on both sides.
- [ ] **`Core\Time::fromIso`** — `crates/mwl-stdlib/src/time.rs:1715`, whose `{err}` is `jiff`'s
      own sentence rather than one this repo writes, so the case pins the prefix and the shape of
      what is quoted back rather than freezing a dependency's wording.

## Backlog

- `Core\Regex\Match::group()`'s "the pattern declares no group" — `crates/mwl-stdlib/src/regex.rs:933`.
- `Core\Math::log`'s non-positive base and `::format`'s decimal cap — `math.rs:1022`, `math.rs:1150`.
- `Core\Arr::average`'s subject with no `decimal` quotient — `arr.rs:4105`.
- `Core\Csv::format`'s non-`string` column and `Core\Out::capture`'s `through` carrier — `csv.rs:512`, `out.rs:141`.
- `Core\Random::bytes`' draw larger than any buffer this process will allocate — `random.rs:333`.
- `Core\Str::wrap`'s multibyte half as an `--ORACLE-DIVERGES--` file — the last `Core\Str` divergence file not written (plan, `Open now`).
