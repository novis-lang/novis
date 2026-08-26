# Handoff

## State

**Conformance is the only frontier left, at 507 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **507 passed, 0 failed** and `mwl test tests/` is **658 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than
it (playbook, *Running things*).

**`python tools/gaps.py --errors` holds nothing a case can take**, and has for three sessions: 58
sites, 57 `fatal` and unreachable by any handler, the one `thrown` left (`csv.rs:512`) unreachable
from source. Every depth slice from here is a judgement call against conventions.md's four case
shapes.

**The count-shaped-allocator family is closed, library and case both.** `Core\Arr::fill`,
`::padStart` and `::padEnd` were the last three that aborted; they draw through the new
`MwlArray::try_reserve` (`crates/mwl-runtime/src/array.rs:789`) now, and `append_copies`
(`crates/mwl-stdlib/src/arr.rs:1321`) takes the member's own name because its `affordable` call
had been hard-coded to `Core\Arr::fill`. The plan's `Open now` owns the shape, the two sentences,
which member reaches which bound, and what the agreement case counts.

**`orient.py`'s `[context] modules` manifest is still wrong** — the same gap the last session
reported, unfixed because no session owns `loop-goal.toml`. It names `registry.rs`, `json.rs`,
`arr.rs` and `regex.rs`; `json.rs` and `regex.rs` can come out and
`crates/mwl-runtime/src/{array,string}.rs` should go in. Nothing else in the pack was missing.

## Next group

Two slices, both over the array representation, and the file set is
`crates/mwl-runtime/src/array.rs` (read only — the claims are behavioural) and
`tests/conformance/core/`. Take them in either order; neither depends on the other.

- [ ] **The packed and hashed forms answer alike** (conventions.md's *invariance over a sweep*) —
      ADR 0007 § 5 says the representation is a statement about keys and not about storage, and
      `crates/mwl-runtime/src/array.rs:190` (`Shape`) is the one place that is true or false.
      Build the same content twice — once as a list, once forced into the hash form by a key the
      packed invariant forbids (`"08"`, a gap, an `unset` in the middle: `array.rs:225`
      `packed_index`, `array.rs:409` `Table::remove`) — and count that every `Core\Arr` reader
      agrees on both, rather than reading one shape's answers off a line.
- [ ] **`unset` does not move the append counter** (conventions.md's *a bound asserted on both
      sides*) — `array.rs:176` (`next_index`) and `array.rs:409`'s doc own the rule:
      `$a = [1,2,3]; unset($a[2]); $a[] = 9;` appends at `3` in *both* shapes, which is PHP 8.5's
      own answer and the differential oracle for it. The bound is the last removal that keeps the
      packed form against the first that degrades it.

## Backlog

- `Core\Str::wrap`'s multibyte half is the last `Core\Str` `--ORACLE-DIVERGES--` file unwritten —
  plan, `Open now`.
- `Core\Path`'s three untwinned members are the largest block `gaps.py --differential` still names.
- A `Core\Json::decodeAs<T>` class carrying the attribute but declaring no field is refused with a
  sentence that is wrong about why — plan, `Open now`.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `docs/agent/loop-goal.toml`'s `[context] modules` needs `crates/mwl-runtime/src/{array,string}.rs`
  and can drop `json.rs` and `regex.rs`.
