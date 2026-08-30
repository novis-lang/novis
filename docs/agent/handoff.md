# Handoff

## State

**Stage 8's corpus count is 992 of 1000** — 8 cases left, all `min_passing` owes. Three landed this
session, all under `tests/conformance/core/` and with no Rust change, all on `Core\Csv`: quoting
minimality (a writer that quoted every field satisfies the existing round-trip case exactly as
well), the `{header: …}` round trip the module doc names as a *composition* of the two members, and
the dialect rule asked of `parse` and `format` together with the `escape` byte only the reader has.
`Core\Csv` has moved from 5 cases to 8.

**The item this session was handed was already on disk, and so was most of the group.** `Core\Uuid`'s
*agreement* slice — one sweep asked of `parse` and `tryParse` together, counted rather than read off
a line — is `uuid-tryparse-and-parse-are-one-reader.nvst`, and
`uuid-every-draw-round-trips-and-a-refusal-quotes-boundedly.nvst` asserts it a second time beside the
draw sweep and the 48-character quote bound. The same held for items 3 and 4: `Core\Hash\Stream`'s
close boundary is in `hash-streams-a-digest-in-chunks.nvst:36-45` and `Core\Math`'s `lcm` and `hypot`
each have a dedicated identity case. **Do not take another group from `gaps.py`'s depth ranking**
without checking each row — the playbook's new *Tooling* bullet owns why it stopped working and what
replaces it.

**Three known gaps carry forward unchanged**, each recorded where its code is: item 18's
`Core\Secret::reveal()` is not in the registry (`nvs_types::expr::quals`); `Live::admit`'s same-class
check is asked of the answer and not of the argument (`crates/nvs-runtime/src/graph.rs` § *Known
gaps*); item 22's `Core\Script` members are unwritten (`crates/nvs-stdlib/src/script.rs`).

**Orientation gaps.** `[context] modules` still has no pattern for `crates/nvs-cli/src/` or
`benches/abi-probe/`, now six sessions old, and none for
`crates/nvs-stdlib/src/{csv,hash,heap,uuid,math,debug,random,validate,bytes}.rs` — every module this
session had to read. `[context] shapes` does not print the `.nvst` *runner* invocation
(`./target/debug/nvs.exe test <path>`), which is in the playbook's *Running things* and is what a
corpus session uses on every case.

## Next group

**The corpus count: 992 to 1000, taken by the method the playbook's new *Tooling* bullet names —
a claim a module's `//!` doc or a `MethodDoc` card argues for, with no case asserting it.** One file
set, `tests/conformance/core/`, and no Rust changes, so several fit under the 120k gate. Check each
with one `grep -rln '<the member>' tests/conformance/core/` before writing; that is the whole triage.

- [ ] **`Core\Bytes` — two qualifier claims, both checked to have no case.** `at`
      (`crates/nvs-stdlib/src/bytes.rs:188`) is the class's one `Contagious` `uint`, so a byte drawn
      out of a tainted buffer stays tainted where `length`, `indexOf` and `compare` answer plain —
      *agreement* over those four asked of one tainted subject. And `unpack`'s format (`:603`) is a
      `Qual::Sink` intrinsic like `pack`'s, so a tainted format is refused **where the call is
      written**: that half is `--EXPECTF-ERROR--`, so it is its own case.
- [ ] **`Core\Validate` — `isEmail` (`crates/nvs-stdlib/src/validate.rs:150`), checked to have no
      case.** All six subjects are `Neutral` and no member returns its subject, so an address
      `isEmail` *accepted* is still tainted afterwards — validation is not laundering. The *edge* is
      that assigning the accepted address to a plain `string` still does not compile, which is again
      `--EXPECTF-ERROR--`.
- [ ] **`Core\Debug` — `dump` 4 cases, `render` 8, and ADR 0033's refusal.** The two members share
      one renderer; *agreement* is that `dump($x)` writes exactly what `render($x)` returns, over a
      table covering every tag. `crates/nvs-stdlib/src/debug.rs` owns the claim.
- [ ] **`Core\Random` — `float` 4, `token` 4, `int` 5.** *A bound asserted on both sides*: `int`'s
      inclusive endpoints and the empty range, and `float`'s half-open interval — the last value it
      can answer and the first it cannot, named together.

## Backlog

- `gaps.py` could rank by *unasserted module-doc claims* rather than by cases per member; the depth
  column has stopped finding room (playbook § *Tooling*).
- `Core\Csv::format` has no `escape` option, so asking for one is a compile error no running case can
  assert — an `--EXPECTF-ERROR--` case if it is worth one (`crates/nvs-stdlib/src/csv.rs:542`).
- Item 18: `Core\Secret::reveal()` is not in the registry — `nvs_types::expr::quals`.
- Item 22: `Core\Script`'s members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- `Live::admit` checks the answer's class, not the argument's — `crates/nvs-runtime/src/graph.rs`
  § *Known gaps*.
- `loop-goal.toml`'s `[context] modules` is missing every `nvs-stdlib` class module a corpus session
  reads, plus `crates/nvs-cli/src/` and `benches/abi-probe/`.
