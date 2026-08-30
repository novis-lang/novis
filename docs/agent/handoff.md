# Handoff

## State

**Stage 8 is closed but for the corpus count.** Its benchmark half landed this session:
`benches/abi-probe/benches/isolation.rs` has a third arm, `isolate/spawn_to_result`, measuring
`nvs_host::Isolate` itself, and `a_spawn_to_result_round_trip_stays_in_the_microsecond_class`
guards it at M5's own number — 10 us, against 0.44 us measured on x86_64-pc-windows-msvc. The
operation both compile is `benches/abi-probe/shared/isolate.rs`, reached by `#[path]` from the bench
and from the guard because this package keeps every compiler crate in `[dev-dependencies]`. What is
left of Stage 8 is `min_passing = 1000` against conformance **986** — 14 cases, and
`python tools/gaps.py` is the worklist.

**The valgrind sweep was red on all 33 fixtures and is green again.** One 56-byte definite loss per
process, from `Compiler::leaked()`: a `Box::leak` that existed only to widen a borrow to the
`&'static` `nvs_runtime::script::install` takes. `script::scoped` is the scoped form — one
`#[expect(unsafe_code)]` whose proof is in its own reason string — and both `nvs run` and `nvs test`
now hold the resolver on their own stack. This was a *regression* in the acceptance sense and it
outranked the group, so it was taken first.

**Three known gaps carry forward unchanged**, each recorded where its code is: item 18's
`Core\Secret::reveal()` is not in the registry (`nvs_types::expr::quals`); `Live::admit`'s same-class
check is asked of the answer and not of the argument (`crates/nvs-runtime/src/graph.rs` § *Known
gaps*); item 22's `Core\Script` members are unwritten (`crates/nvs-stdlib/src/script.rs`).

**Orientation gaps, now four sessions old:** `[context] modules` still has no pattern for
`crates/nvs-cli/src/` and none for `benches/abi-probe/`. Both were needed again this session.

## Next group

**The corpus count: 986 to 1000, taken as the four thinnest classes `gaps.py` ranks.** One file set,
`tests/conformance/core/`, and no Rust changes — so several fit in one session under the 120k gate.
`python tools/gaps.py` re-derives the ranking; these are its top rows at this commit, and each names
the *shape* the case should take from conventions.md's four (edges, invariance over a sweep, a bound
asserted on both sides, agreement).

- [ ] **`Core\Task\Channel` is the thinnest class in the tree** — depth 4.0, floor 4, and `close` and
      `send` are its least-asked members (`crates/nvs-stdlib/src/channel.rs`). ADR 0072 § 4's "nothing
      is still running when the call returns" is the invariant; a *bound asserted on both sides* — the
      last `send` a bounded channel accepts and the first one that suspends — is the shape with the
      most room.
- [ ] **`Core\Serialize`, depth 6.0 over two members** (`decode` 6, `encode` 6,
      `crates/nvs-stdlib/src/serialize.rs`). ADR 0023 § 2 is the specification and its § 2 is already
      in the goal's `[context] adrs`; the untested half is *refusal* — bytes that are not Novis's own
      format, and a class whose declared properties no longer match.
- [ ] **`Core\Debug`, depth 5.0 with `dump` at 3** (`crates/nvs-stdlib/src/debug.rs`). ADR 0033's
      refusal is the edge worth pinning: a `secret` value reaching `dump` or `render`.
- [ ] **`Core\Csv` and `Core\Hash\Stream`, both depth 5.0 over 5 cases**
      (`crates/nvs-stdlib/src/{csv,hash}.rs`). `gaps.py --errors` lists `csv.rs:610`'s
      `Core\Csv::format()` throw as unasserted — a `thrown`, so a case can catch and echo it.

## Backlog

- A `secret` value *inside an array* crosses both copy carriers unrefused — `quals::is_secret` reads
  the top-level qualifier only (`crates/nvs-types/src/expr/quals.rs` should own the note).
- An object does not cross the isolate boundary even when both files declare the identical class;
  same root as `Live::admit` (playbook, *Writing a test case*).
- `Core\Task::map`'s `fn` argument accepts a `callable` variable where `::all`'s field is E0774;
  ADR 0072 § *Verification* names only the `all` side, so nothing says whether that is intended.
- `[context] modules` needs `crates/nvs-cli/src/` and `benches/abi-probe/` (`docs/agent/loop-goal.toml`).
- M5's acceptance still owes 100k concurrent tasks, a deliberate deadlock, `Core\Task::map` near-linear
  across cores and a ThreadSanitizer-clean run (`python tools/plan.py --show M5:verify`).
