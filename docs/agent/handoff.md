# Handoff

## State

**Conformance is the only frontier left, at 499 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **499 passed, 0 failed** and `mwl test tests/` is **650 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than
it (playbook, *Running things*).

**`python tools/gaps.py --errors` is the worklist and it now prints 66 sites, of which 59 are
`fatal` and unreachable by any case, leaving 7.** This session closed `Core\Math`'s two guarded
arguments (`log`'s base, `format`'s decimal cap) and `Core\Time::fromIso`, one new case each. The
seven left are `json.rs:509` and `json.rs:741`, `regex.rs:933`, `arr.rs:4105`, `csv.rs:512`,
`out.rs:141` and `random.rs:333`. The one site still hidden behind a `{member}` hole is
`time.rs:1968`, and it is still **not** owed a case, for the reason the plan's `Open now` gives.

**One behaviour changed rather than only being asserted:** `Core\Time::fromIso` now throws
`ParseError` and not the bare `RuntimeError` a plain `Fault::thrown` gives, on the rule
`Core\Time::parse` already splits its two classes by. The reason is in that helper's own doc
comment; the plan's `Open now` says what the case asserts past the class.

**`math.rs` and `time.rs` are out of `orient.py`'s `[context] modules`** now that neither holds an
assertable site, and `json.rs` and `arr.rs` are in — the manifest names `registry.rs`, `json.rs`,
`arr.rs` and `regex.rs`, which is exactly the group below.

## Next group

These are refusal slices and, as before, they do not cluster by file: what all three share is
`tests/conformance/core/` and one **read-only** `mwl-stdlib` module each, one `peek.py` apiece, and
all three modules are in the `[context] modules` manifest already. Slice 1's two sites are one file,
which is why it goes first. Every one is the *Edges* shape over a `Fault::`, closed with a counted
claim and with the bound named on both sides where there is one.

- [ ] **`Core\Json`'s two `thrown_as`** — `crates/mwl-stdlib/src/json.rs:509` (`Core\Json::encode():
      {why}`) and `crates/mwl-stdlib/src/json.rs:741` (`Core\Json::decodeAs(): `{}` has no JSON
      codec — a class participates by carrying …`). ADR 0071 owns both: § 5 the one-throw-lists-every-
      bad-field rule, and the derive attribute is what decides whether a class has a codec at all.
      The counted claim is which *values* `encode` refuses against the ones it renders — the plan
      records that the decoder reads a scalar-fielded class only, so the classes without a codec are
      a partition, not a list. Check the class each throws as: `thrown_as` names one deliberately.
- [ ] **`Core\Regex\Match::group`** — `crates/mwl-stdlib/src/regex.rs:933` (`Core\Regex\Match::group():
      the pattern declares no group `{}``). Spec § 5 owns the row. The claim already stated in the
      plan is `group`'s three-way split — not declared (throws), declared and did not participate
      (`null`), participated and captured nothing (`""`) — so the counted claim is that split over a
      pattern's whole group set, against `groups()`, which the existing case pins under
      `PREG_UNMATCHED_AS_NULL`'s reading. A name and its number reach the same group.
- [ ] **`Core\Arr::average`'s exact answer** — `crates/mwl-stdlib/src/arr.rs:4105` (`Core\Arr::average
      has no `decimal` answer for this subject: the quotient is …`). Spec § 2 owns the row and ADR
      0054 the 16-byte `decimal`. The bound is on both sides — the widest subject whose quotient is
      exact against the first one that is not — and the neighbouring `arr.rs:4098` `fatal` (more
      entries than a `uint` counts) is unreachable and owed nothing.

## Backlog

- `csv.rs:512`, `out.rs:141` and `random.rs:333` — the last three assertable `--errors` sites after
  the group above; `docs/agent/handoff.md` is where they are picked up.
- `Core\Str::wrap`'s multibyte half, the last `Core\Str` `--ORACLE-DIVERGES--` file not written
  (plan, `Open now`).
- `gaps.py --differential` is 8, with `Core\Path`'s three the largest block (plan, `Open now`).
- `Core\Json::decodeAs<T>`'s decoder reads a scalar-fielded class only (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row
  (`mwl_stdlib::hash`'s module doc).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
