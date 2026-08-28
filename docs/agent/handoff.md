# Handoff

## State

**M4's Stage 8, and `Core\Test` is finished** — every member of the class now has a depth case
beyond its coverage one, and `python tools/gaps.py` ranks it at 2.89 cases per member, off the
frontier. The tree is at **771 conformance plus 189 differential**. Three cases landed, all over
`tests/conformance/core/`, and the whole named group is done.

- **Every scalar arm of `shown` renders the value as itself.** `null`, `false`/`true`, a signed
  `int`, a `uint` past `i64::MAX`, a `float` through Rust's `to_string` (`1.0` prints `1`) and a
  `decimal` keeping its written scale (`1.00` prints `1.00`). The `int` and the `uint` of the same
  digits are asserted to agree by pulling both renderings back out of the two messages and running
  `assertSame` on them, so a tag reaching the rendering fails while each line still looks right.
- **`assertEquals` refuses an object whose class declares no `compareTo`**, quoting the class
  through `shown` and naming `assertEqualsDeep` and `assertSame`; both of those are then run on
  the very object the refusal named, one agreeing and one not.
- **`expectFailure` reads the ledger, not the throw.** A body that swallowed its own failed
  assertion in a `catch` is still discharged; a body that ran clean is itself a failure
  (`the callable ran without a failed assertion`), catchable like any other. The ledger is not
  observable from a program otherwise — no member reports it — so that swallowing row is the whole
  evidence available for "the record, not the control flow".

## Next group

**`Core\Math`'s agreements and bounds** — the file set is `crates/nvs-stdlib/src/math.rs` plus
`tests/conformance/core/`. It is the thinnest class on `gaps.py` (38 members, exactly one case
each), so what every slice adds is the boundary or the invariant, never another row.

- [ ] **The four roundings agree on an integer and part only away from zero** (`math.rs:89` `ceil`,
      `:96` `floor`, `:103` `truncate`, `:110` `round`) — the *agreement* shape: one question asked
      of all four over a swept table of `float`s, asserting they **agree** on every integral input
      and on every positive fraction, and separating exactly on the negative ones, where `floor`
      and `truncate` part. Count the agreements rather than reading the rows.
- [ ] **`toBase` and `fromBase` round-trip, and refuse the same bounds** (`math.rs:299`, `:306`,
      the `{base:}` option at `:444`) — the *bound asserted on both sides* shape: the lowest and
      highest base the pair accepts, named together with the first refused one on each end, plus
      one round trip per accepted base so a member that stopped one entry early cannot print
      plausibly against either half alone.
- [ ] **The inverse members answer their domain edge rather than plausibly past it** (`math.rs:201`
      `asin`, `:208` `acos`, `:257` `acosh`, `:264` `atanh`, with `:285` `isNan` to read the
      answer) — the *edges* shape. Check first whether an out-of-domain operand throws or answers
      `NAN`; the playbook's rule about judging a refusal site before writing a case applies, and a
      scratch `.agent-tmp/*.nvs` settles it in one run.

## Backlog

- `crates/nvs-stdlib/src/csv.rs:512`'s unreachable `thrown` still owes no case — `docs/agent/playbook.md`.
- 67 other unasserted error paths, 65 of them `Fault::fatal` and mostly internal — `python tools/gaps.py`.
- `Core\Uri` and `Core\Time\DateTime` are the next two thinnest after `Core\Math` — `python tools/gaps.py`.
- 54 guard-test names in `loop-goal.toml` match nothing `cargo test` runs — `docs/agent/guard-name-debt.md`.
