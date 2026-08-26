# Handoff

## State

**Conformance is the only frontier left, at 502 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **502 passed, 0 failed** and `mwl test tests/` is **653 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than
it (playbook, *Running things*).

**`python tools/gaps.py --errors` is the worklist and it now prints 61 sites, of which 57 are
`fatal` and unreachable by any case, leaving 4.** This session closed `Core\Json`'s two `thrown_as`
and `Core\Regex\Match::group`, one new case file each for the first two claims and one for the
third. The four left are `arr.rs:4105`, `csv.rs:512`, `out.rs:141` and `random.rs:333` — no file
holds more than one. Three sites are hidden behind a stem rather than closed: `time.rs:1968` as
before, and now `json.rs:767` and `json.rs:869`, both `fatal` and both behind the
`Core\Json::decodeAs(): ` prefix the codec case asserts. None is owed a case; the plan's `Open now`
says why.

**Two `Core\Json::encode` messages changed rather than only being asserted**, both in the arm a
program actually reaches: a non-finite `float` is now quoted the way `echo` spells it (`NAN`,
`INF`, `-INF`) through `mwl_runtime::php_float_to_string` rather than in Rust's `inf`, and a value
whose tag has no JSON spelling is named by `Tag::describe` — ``a `bytes` value has no JSON
encoding`` — where it used to report a tag *number*. That arm is where a `bytes` argument lands,
which `Encodable::text`'s own doc comment already said.

**`orient.py`'s `[context] modules` manifest is now wrong for the group below**: it names
`registry.rs`, `json.rs`, `arr.rs` and `regex.rs`, and the three slices after `Core\Arr::average`
read `csv.rs`, `out.rs` and `random.rs`, none of which it selects. `json.rs` and `regex.rs` can
come out at the same time.

## Next group

These four are the whole of what `--errors` still lists as catchable. They do not cluster by file:
what they share is `tests/conformance/core/` and one **read-only** `mwl-stdlib` module each, one
`peek.py` apiece. Every one is the *Edges* shape over a `Fault::thrown`, closed with a counted
claim and with the bound named on both sides where there is one. Only the first module is in the
`[context] modules` manifest today.

- [ ] **`Core\Arr::average`'s exact answer** — `crates/mwl-stdlib/src/arr.rs:4105` (`Core\Arr::average
      has no `decimal` answer for this subject: the quotient is …`). Spec § 2 owns the row and ADR
      0054 the 16-byte `decimal`. The bound is on both sides — the widest subject whose quotient is
      exact against the first one that is not — and the neighbouring `arr.rs:4098` `fatal` (more
      entries than a `uint` counts) is unreachable and owed nothing.
- [ ] **`Core\Csv::format`'s non-`string` column** — `crates/mwl-stdlib/src/csv.rs:512`
      (`Core\Csv::format(): column {column} of a row holds a value that is not a `string``). Spec
      § 9 owns the row. The counted claim is which *tags* a row may hold against the ones it may
      not, and the message names the column index, so assert it at more than one column in turn —
      the shape `Core\Encoding`'s offset rows already use.
- [ ] **`Core\Out::capture`'s `through`** — `crates/mwl-stdlib/src/out.rs:141` (``Core\Out::capture`'s
      `through` must answer a `{}`, and this one answered tag …`). The callback's *return* is what
      is refused, so the claim is that the option is checked per call rather than at the boundary,
      and `examples/collect.mwl` is the fixture that already exercises the accepting side.
- [ ] **`Core\Random::bytes`'s draw bound** — `crates/mwl-stdlib/src/random.rs:333`
      (`Core\Random::bytes(): the requested draw is larger than any buffer this process …`). Name
      the bound on both sides if the accepted side is reachable in a test's time and memory; if it
      is not, say so in the case comment and assert the refusal plus the largest draw that is
      cheap.

## Backlog

- A class carrying `#[Core\Json\Derive]` but declaring **no field** refuses in both directions with
  the sentence that says it does not carry the attribute — plan, `Open now`.
- `Core\Regex\Match::group`'s refusal is a bare `RuntimeError` where the call-site-argument rule
  that made `Core\Time::parse`'s bad pattern a `LogicError` argues for `Logic` — plan, `Open now`.
- `python tools/gaps.py --differential` still names 8 members with a PHP twin and no oracle case;
  `Core\Path`'s three are the largest block — plan, `Open now`.
- `Core\Str::wrap`'s multibyte half is the last `Core\Str` `--ORACLE-DIVERGES--` file unwritten —
  plan, `Open now`.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
