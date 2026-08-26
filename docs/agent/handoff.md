# Handoff

## State

**Conformance is the only frontier left, at 504 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **504 passed, 0 failed** and `mwl test tests/` is **655 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than
it (playbook, *Running things*).

**`python tools/gaps.py --errors` is effectively exhausted as a worklist.** It prints 59 sites, of
which 57 are `fatal` and unreachable by any case; of the two `thrown` left, `csv.rs:512` cannot be
reached from source at all — the plan's `Open now` says why, and the playbook's *Writing a test
case* holds the check to run before taking a site as catchable — so `random.rs:333` is the only
one, and it sits behind `mwl_runtime::affordable`'s own refusal. **Every depth slice after it is a
judgement call against conventions.md's four case shapes, not a tool's output.**

**One behaviour changed this session:** `Core\Out::capture`'s `{through:}` used to accept any
object, so a closure answering a foreign one had it handed back under the member's declared
`Core\Cli\Text`. It compares the class name now (`out.rs`'s `not_the_carrier`), and its message
names a non-object by its type rather than by a tag number.

**`orient.py`'s `[context] modules` manifest is wrong for both the last group and the next.** It
names `registry.rs`, `json.rs`, `arr.rs` and `regex.rs`; this session needed `out.rs`, `csv.rs`,
`cli.rs` and `crates/mwl-runtime/src/decimal.rs`, and the group below needs `random.rs` and
`crates/mwl-runtime/src/abi.rs`. `json.rs`, `arr.rs` and `regex.rs` can come out.

## Next group

One module, read-only, and one directory: `crates/mwl-stdlib/src/random.rs` and
`tests/conformance/core/`. Spec § 11 owns the rows. Take them in this order — the first is the
last site `--errors` names, and the two after it are the *Agreement* and *bound-on-both-sides*
shapes over the same seven members, so the file is already open.

- [ ] **`Core\Random::bytes`'s draw bound** — `crates/mwl-stdlib/src/random.rs:333`, against
      `mwl_runtime::affordable`'s own refusal at `crates/mwl-runtime/src/abi.rs:161`. Two checks
      run in a row and they answer different questions (the comment at `random.rs:325` says so);
      what the case pins is the bound *between* them, so read `affordable`'s cap first — site 333
      is reachable only for a count `affordable` allows and the allocator cannot serve. If it is
      unreachable, say so in the plan the way `csv.rs:512` is said now, and take the next slice.
- [ ] **`Core\Random`'s count-shaped refusals agree** — `random.rs:268`, `:319`, `:366`, `:438`,
      all `thrown` and all already asserted a row at a time. The claim left is *Agreement*: the
      seven members (`int`, `float`, `bytes`, `token`, `pick`, `sample`, `shuffle`, at
      `random.rs:80`-`:122`) refuse an empty subject and a zero count the same way, asserted by
      counting that they agree rather than by what each answered.
- [ ] **`Core\Random::int`'s range is inclusive at both ends** — `random.rs:268`. The last
      accepted range against the first refused one, plus the single-value range, counted over a
      sweep so a member that stops one short still fails.

## Backlog

- `csv.rs:512` becomes reachable only when ADR 0007 § 2's `array<T> as array<U>` lowers —
  `mwl-ir`'s known gaps, panic site `crates/mwl-ir/src/lower/expr.rs:877`.
- `Core\Str::wrap`'s multibyte `--ORACLE-DIVERGES--` file, the last `Core\Str` divergence not
  written — plan, `Open now`.
- `Core\Path::basename`/`dirname`/`normalize`'s three PHP twins — `path.rs:447`, `:478`, `:664`,
  `gaps.py --differential`. Off the acceptance path: that gate is met at 151 of 150.
- `Core\Arr::flattenDeep`, `Core\Json::decode`/`isValid` and `Core\Math::gcd`/`lcm` — the rest
  of the same list; the `gmp` pair has no callable twin on either leg.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row — plan,
  `Open now`.
- `do`/`while` is the one M4 control-flow statement that does not lower — plan, `Open now`.
