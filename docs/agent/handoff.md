# Handoff

## State

**The acceptance regression is closed, and it was never a leak.** `valgrind examples/serialize.nvs`
reported 96 bytes in two blocks; both are the fixture's own reference cycles, which a refcounting
runtime with no cycle collector cannot reclaim. The fixture breaks its rings before it exits, its four
frozen lines are unchanged, and the sweep is green. The playbook's *Running things* bullet owns the
shape so the next session does not re-open `graph.rs` for it.

**Stage 8's corpus count is 989 of 1000** — 11 cases left, all `min_passing` owes. Three landed this
session, all under `tests/conformance/core/` and with no Rust change: `Core\Task\Channel`'s
close-boundary refusal (`send` after `close` is a fatal no `catch` sees) and its capacity sweep, and
`Core\Debug::dump`'s variadic arity boundary (no arguments writes nothing at all). `Core\Task\Channel`
has moved from the thinnest class in the tree, depth 4.0, to 6.0.

**A handoff's ranking goes stale the moment a case lands, so re-run `python tools/gaps.py` rather than
taking the group's order on faith.** Last session's item 2, `Core\Serialize`, was already at gaps.py's
floor before this session opened, and `Core\Arr`, `Core\Math`, `Core\Bytes` and `Core\Uuid` all rank
above it now. The group below is that tool's current top rows, not last session's.

**Three known gaps carry forward unchanged**, each recorded where its code is: item 18's
`Core\Secret::reveal()` is not in the registry (`nvs_types::expr::quals`); `Live::admit`'s same-class
check is asked of the answer and not of the argument (`crates/nvs-runtime/src/graph.rs` § *Known
gaps*); item 22's `Core\Script` members are unwritten (`crates/nvs-stdlib/src/script.rs`).

**Orientation gaps.** `[context] modules` still has no pattern for `crates/nvs-cli/src/` and none for
`benches/abi-probe/`, now five sessions old. New this session: nothing selects
`crates/nvs-stdlib/src/{channel,debug,uuid,csv,hash,math}.rs`, `crates/nvs-test/src/lib.rs` (the
`.nvst` section table, which a case using `--EXPECTF-ERROR--` needs) or
`crates/nvs-runtime/src/object.rs` — the last of which owns the decision the acceptance failure turned
on.

## Next group

**The corpus count: 989 to 1000, taken as the thinnest classes `gaps.py` ranks *now*.** One file set,
`tests/conformance/core/`, and no Rust changes — so several fit in one session under the 120k gate.
Each names the shape from conventions.md's four with the most room left on that class.

- [ ] **`Core\Uuid` — depth 5.0, floor 3, 11 cases over 5 members.** `tryParse`
      (`crates/nvs-stdlib/src/uuid.rs:148`), `parse` (`:139`), `v7` (`:130`). *Agreement*: one sweep of
      inputs asked of `parse` and `tryParse` together, asserting the two never disagree about what is a
      UUID rather than what either answered — a `tryParse` that grew its own validation fails here and
      looks right on its own line.
- [ ] **`Core\Csv` — depth 5.0 over 5 cases and two members.** `parse`
      (`crates/nvs-stdlib/src/csv.rs:145`), `format` (`:154`). *Invariance over a sweep*: every quoting
      edge — an embedded delimiter, a quote, a newline, a leading space, an empty field — round-trips
      through `format` then `parse`, counted rather than read off a line.
- [ ] **`Core\Hash\Stream` — depth 5.0 over 5 cases.** `update`
      (`crates/nvs-stdlib/src/hash.rs:447`), `finish` (`:456`). *Agreement*: the same bytes fed in one
      chunk, in two, and byte at a time must all equal the one-shot digest, so a stream that lost or
      double-counted a boundary fails while every single chunking still hashes plausibly.
- [ ] **`Core\Math` — depth 5.0, floor 3.** `lcm` (`crates/nvs-stdlib/src/math.rs:165`), `hypot`
      (`:192`), `atan2` (`:273`). *Edges*: `python tools/gaps.py --errors` lists the boundaries none of
      these three is asked about.

## Backlog

- `Core\Serialize` and `Core\Task\Channel` are both at gaps.py's floor now — they come back only if it
  rises (`python tools/gaps.py`).
- Item 18: `Core\Secret::reveal()` is not in the registry — `nvs_types::expr::quals`.
- Item 22: `Core\Script`'s members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- `Live::admit` checks the answer's class, not the argument's — `crates/nvs-runtime/src/graph.rs`
  § *Known gaps*.
- `[context] modules` needs patterns for `nvs-cli`, `benches/abi-probe`, the `nvs-stdlib` member
  modules and `nvs-runtime/src/object.rs` — `docs/agent/loop-goal.toml`.
