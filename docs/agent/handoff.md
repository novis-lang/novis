# Handoff

## State

**Conformance is the only frontier left, at 492 of 600; the differential gate is met at 151 of the
150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **492 passed, 0 failed** and `mwl test tests/` is **643 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than it
(playbook, *Running things*).

**`python tools/gaps.py --errors` is the worklist and it is down to 84 sites from 91**, of which
**63 are `fatal` and unreachable by any case, leaving 21 a case can catch**. `str.rs` and
`encoding.rs` now hold none of them: this session closed `Core\Encoding`'s `encodeText`/`decodeText`
charset pair and all five `Core\Str` refusals, in one new case each. The plan's `Open now` says what
the two cases assert past their rows.

**One library change rode along**: `Core\Str::at`'s message was the only one in `mwl-stdlib`
missing the `()` every sibling writes, so it now reads `Core\Str::at():`. Nothing asserted the old
spelling, but it means the release binary must be current before that case is believed.

**Spec § 7 owns `Core\Encoding`, not § 8** — § 8 is `Core\Path`. The previous session's
`encoding-each-decoder-says-why-it-refused-and-where.mwlt` cited § 8 in its `--TEST--` line and its
lead comment; both are corrected, and the handoff item that said this work "closes spec § 8" meant
§ 7.

**`orient.py`'s `[context] modules` needs `time.rs` and `math.rs`**, which the next group reads and
which did not print. `path.rs` is already in it.

## Next group

Each slice reads **one** `mwl-stdlib` module for its messages and writes **one** new file under
`tests/conformance/core/` — that directory is the file set they share, and the modules are read-only
here, so a second slice costs one `peek.py` of its anchors. Every one is the *Edges* shape over a
`Fault::thrown`, closed with a counted claim and with the bound named on both sides where there is
one. `gaps.py --errors` already holds these anchors; do not re-derive them. Slices 2 and 3 share
`time.rs`, so they are the cheap pair to take together.

- [ ] **`Core\Path::withExtension`'s four** — `crates/mwl-stdlib/src/path.rs:529` (an empty
      extension, where `null` is how a caller removes one), `:535` (written with its dot), `:541`
      (contains a path separator) and `:550` (a path that names no file). Spec § 8 owns the rows.
      A built path must normalize `Path::SEPARATOR` or assert something separator-free, per the
      goal's standing decisions — but all four of these are refusals, so none renders a path.
- [ ] **`Core\Time`'s three `format` refusals** — `crates/mwl-stdlib/src/time.rs:2155`
      (`DateTime::format`), `:2681` (`Date::format`) and `:2876` (`TimeOfDay::format`). One `why`
      behind three members is the *Agreement* shape outright: assert that the three answer the same
      thing for the same bad pattern rather than what each answered.
- [ ] **`Core\Time`'s zone and parse refusals** — `crates/mwl-stdlib/src/time.rs:1844`
      (`Zone::of`, unknown id), `:1968` (`DateTime::{member}`, the same id reached through a second
      member, so the two agree), `:1715` (`fromIso`) and `:2464`/`:2469` (`Time::parse`, both
      `thrown_as`). Spec § 4 owns the rows.

## Backlog

- `Core\Math`'s two — `math.rs:1022` (`log`'s base not greater than zero), `:1150` (`format` past
  its decimals cap). Needs `math.rs` in `[context] modules`.
- `Core\Json`'s two `thrown_as` — `json.rs:509` (`encode`), `:741` (`decodeAs` with no codec).
- `Core\Regex\Match::group` on an undeclared group — `regex.rs:933`; `Core\Csv::format` on a
  non-`string` column — `csv.rs:512`.
- `Core\Out::capture`'s `through` answer — `out.rs:141`; `Core\Random::bytes`' draw cap —
  `random.rs:333`; `Core\Arr::average`'s missing `decimal` quotient — `arr.rs:4105`.
- `Core\Str::wrap`'s multibyte half is the last unwritten `--ORACLE-DIVERGES--` file — the plan's
  `Open now` owns it.
- 63 of the 84 `--errors` sites are `fatal` and no case reaches them, so depth past the low 500s
  comes from conventions.md's four shapes rather than from more refusal messages.
