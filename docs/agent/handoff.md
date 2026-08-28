# Handoff

## State

**M4's Stage 8, depth.** The tree is at **815 conformance plus 189 differential**. Nothing is
blocked.

`Core\Test` is done as a depth target — `gaps.py` now ranks it 4.0/floor 3, out of the thin end.
Two cases landed, both sweeps rather than more rows. `assertThrows`'s class argument is pinned as
a **bound with a direction**: one thrown `ParseError` held against `ParseError`, `RuntimeError`
and `Throwable` (ancestry matches upward, `Ctx::pending_conforms_to`'s own decision note), and
refused against the `ParseError` a thrown `RuntimeError` does not descend to, against the sibling
`IOError`, and against a body that returned — four refusals counted, with the option bag shown
prefixing a report it does not otherwise decide. And `assertSame`/`assertCount`/`assertDoesNotThrow`
are asserted to **agree on what reaches § 5's ledger**: a failure records whether its throw
propagates or the body swallows it (six discharges through `expectFailure`, counted), and a held
assertion leaves no failure to discharge (three refusals, counted).

The third slice of the last group is **closed with no case**: `assertCount`'s two `Fault::fatal`
sites cannot be reached from source at all, its parameters being `array<T>` and `uint`. That is
now a playbook bullet under *Divergences and refusals already pinned*.

One thing a program cannot observe and the next session should not try to: nothing in `Core\Test`
reports a ledger tally, and `expectFailure` discharges only *failed* entries
(`Ctx::discharge_failures_from`), so a **passing** entry's own record is visible only to
`nvs_cli::runner` under § 20. A case claiming to count passes would be claiming more than it sees.

The gap six handoffs back still stands: **no `Core` class reaches `nvs_hir::implements_interface`**,
so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s is `E0411`. It is in the backlog
and still deserves a session of its own.

## Next group

**`Core\Random`, the thinnest class `gaps.py` now ranks** (depth 3.0, floor 2, 7 cases over 7
members) — one file set: `crates/nvs-stdlib/src/random.rs` and `tests/conformance/core/`. Nothing
here needs `test.rs` open. Seven cases already exist under `tests/conformance/core/random-*`; read
their `--TEST--` lines first so a new one adds a boundary rather than another row.

- [ ] **`Core\Random::float` is bounded on both sides of the unit interval**
      (`crates/nvs-stdlib/src/random.rs:285`, `args: [0]`) — one case, the *bound* shape. It takes
      no arguments at all, so the only thing to assert is the interval: sweep a few hundred draws
      and count that every one satisfies `>= 0.0` and the half-open `< 1.0`, which is the half a
      member built on a closed division gets wrong. Float `<`, `>` and `&&` all lower (playbook).
- [ ] **`Core\Random::pick` over the degenerate array** (`random.rs:428`) — one case, the *edge*
      shape. The one-element array (the only draw whose answer is determined), and the empty one,
      whose refusal's `Fault::` constructor decides whether the case can `catch` it — check the
      constructor at the site before assuming, per the playbook.
- [ ] **`Core\Random::shuffle` preserves the multiset it was given** (`random.rs:485`) — one case,
      the *invariance over a sweep* shape. Sort the shuffled result and assert it equals the sorted
      input, counted over a table of lengths including 0 and 1, so a member that drops or
      duplicates an element fails while any single shuffle still looks plausible.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `Core\Uri::compareTo` exists while
  `$a < $b` over two `Uri`s is `E0411` — its own session; owned by ADR 0013.
- `Core\Debug` is the other 3.0/floor-2 class (`dump` 2, `render` 4) — the group after `Core\Random`.
- `Core\Str::fold`, `graphemes` and `indexOf` have one case each, the tree's thinnest members —
  `gaps.py`, and the playbook's `mbstring` bullet applies to their oracles.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `array<T> as array<U>` is the conversion row `nvs-ir` still panics on (`lower/expr.rs:877`) —
  ADR 0007 § 2 owns the row.
