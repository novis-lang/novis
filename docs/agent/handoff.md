# Handoff

## State

Goal `core-env-and-4-more`, 3 of its 16 features taken. `Core\Env::all`, `get` and `mode` each carry
`about.md`, three examples with blessed `.out` files, an attack, a bench and a Rust `// covers:` test
(`crates/nvs-stdlib/src/env.rs`'s `mod tests`). What they still owe is the perf figure alone, which
`python tools/dossier.py --record-perf` writes against a release build; the last goal recorded its
members' figures in one commit at its end, and this one does the same.

The `mode` bench found one allocation per call: `nvs_config::Request::mode` answered a `String`. It
now answers a `Cow<'_, str>` borrowed from the overlay or the snapshot, and the bench declares
`allocations 0`, which pins it.

## Next group

**`Core\Fatal`'s two members** — one file set: `crates/nvs-stdlib/src/fatal.rs`, plus the three proof
trees under `core/Fatal/`. Both register a handler that runs after the program has stopped, so an
example has to end the program (a limit or an uncaught throw) to show anything, and the attack must
say `// hostile: ends-early`. `rule:errors/on-limit` and `rule:errors/on-uncaught-throw` own them.

- [ ] **`Core\Fatal::onLimit`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/fatal.rs:47`
- [ ] **`Core\Fatal::onUncaughtThrow`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/fatal.rs:59`

## Backlog

- The perf figures for all 16 of this goal's members, recorded once at the goal's end with
  `python tools/dossier.py --record-perf` — `benches/members/README.md`.
- `Core\Env::get`'s throw on a value that is not UTF-8 is pinned by no test, because setting such a
  variable needs `std::env::set_var` in a test binary that reads the environment on other threads —
  `crates/nvs-stdlib/src/env.rs`'s module doc.
- Then `Core\Hash` (`crates/nvs-stdlib/src/hash.rs:305`) and `Core\Heap` (`crates/nvs-stdlib/src/heap.rs:109`).
