# Handoff

## State

**Conformance is the only frontier left, at 490 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **490 passed, 0 failed** and `mwl test tests/` is **641 passed,
0 failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than
it (playbook, *Running things*).

**`python tools/gaps.py --errors` is the worklist and it is down to 91 sites from 100**, of which
63 are `fatal` and unreachable by any case. Five of the nine closed this session were closed by a
*tooling* fix, not by a case: `gaps.py`'s quoted-literal regex stopped at the first `"` even when
it was an escaped one, so every message that quotes its own operand back — the four `Core\Encoding`
decoders and `Core\Uuid::parse` — had a stem of `Core\Encoding::fromBase64(): \`, which no case
could ever contain. A `--errors` row is trustworthy again; the plan's `Open now` says what the four
new-case sites now assert.

**`crates/mwl-stdlib/src/bytes.rs` is finished as far as a case can take it.** Its four `thrown`
sites are asserted by message and its four `Fault::fatal` ones are not case-assertable — check the
`Fault::` constructor at a site before planning a case around it (playbook, *Writing a test case*).
`encoding.rs` keeps only the `encodeText`/`decodeText` charset pair, which is item 1 below.

**`orient.py`'s `[context] modules` was short again and this session fixed it rather than
reporting it**: `arr.rs` is out (that section is closed) and `encoding.rs` and `path.rs` are in, so
the next group's three modules all print. `bytes.rs` was never added and no longer needs to be.

## Next group

Each slice reads **one** `mwl-stdlib` module for its messages and writes **one** new file under
`tests/conformance/core/` — that directory is the file set they share, and the modules are
read-only here, so a second slice costs one `peek.py` of four anchors. Every one is the *Edges*
shape over a `Fault::thrown`, closed with a counted claim and with the bound named on both sides.
`gaps.py --errors` already holds these anchors; do not re-derive them. Take them in this order.

- [ ] **`Core\Encoding`'s charset pair** — `crates/mwl-stdlib/src/encoding.rs:755`
      (`encodeText`: a character the target charset has no spelling for, its message naming the
      code point) and `:785` (`decodeText`: a byte sequence at an offset that is not that charset).
      This empties `encoding.rs`'s reachable list and closes spec § 8. Spec § 8 owns the rows;
      `encoding-text-trio-converts-through-a-charset.mwlt` is the shallow case to leave in place.
- [ ] **`Core\Str`'s five refusals** — `crates/mwl-stdlib/src/str.rs:785` (`at` outside the string,
      counted in *characters*, which is the divergence from `Core\Bytes::at`'s bytes), `:955`
      (`chunk` size below 1), `:1569` (`countOf`'s empty needle), `:1907` and `:1912` (`wrap`'s
      empty `breakWith`, and a width of 0 that must cut). Spec § 1 owns the rows.
- [ ] **`Core\Path::withExtension`'s four** — `crates/mwl-stdlib/src/path.rs:529` (empty
      extension, the message naming `null` as the removal spelling), `:535` (written without its
      dot), `:541` (contains a separator) and `:550` (a path naming no file). Spec § 5 owns the
      rows; a built path must be normalized or separator-free (loop-goal, *Path and the two legs*).

## Backlog

- `arr.rs:4105` — `Core\Arr::average` has no `decimal` answer for a subject (`gaps.py --errors`).
- `time.rs` × 6 and `json.rs` × 2 are the largest remaining `thrown` block (`gaps.py --errors`).
- `Core\Str::wrap`'s multibyte half is the last unwritten `--ORACLE-DIVERGES--` file (plan, `Open now`).
- 8 differential twins left, `Core\Path`'s three the largest block (`gaps.py --differential`).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
- 63 `Fault::fatal` sites are on `--errors` and none is case-assertable; judge before planning one.
