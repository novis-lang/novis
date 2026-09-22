# Handoff

## State

Goal `core-env-and-4-more`, 11 of its 16 features taken. `Core\Env::all`, `get` and `mode`,
`Core\Fatal::onLimit` and `onUncaughtThrow`, `Core\Hash::of`, `hmac`, `equals` and `stream`, and
`Core\Hash\Stream::update` and `finish` each carry `about.md`, three examples with blessed `.out`
files, an attack, a bench and a Rust `// covers:` test. The only proof they still owe is the perf
figure. `python tools/dossier.py --record-perf` writes it against a release build, and this goal
records all of them in one commit at its end.

`Core\Hash\Stream::finish` now feeds each retained chunk to the running hasher where it lies
(`crates/nvs-stdlib/src/hash.rs`'s `Running`), so a stream fed one buffer many times is held once;
`tests/conformance/core/hash-stream-finish-holds-no-copy-of-what-it-was-fed.nvst` pins it.

Goal `limit-handler-reach`'s stage 3 still owns the `cpu_time` narrowed by `Core\Config::set`
that is accepted and never enforced.

## Next group

**`Core\Heap`'s five members** — one file set: `crates/nvs-stdlib/src/heap.rs`, plus the proof
trees under `core/Heap/<member>` (`python tools/dossier.py --id` prints each path).
`rule:testing/feature-proofs` owns what each owes. `push`, `pop` and `peek` share one set of
examples well; `count` and `isEmpty` are small enough to take in the same session.

- [ ] **`Core\Heap::push`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/heap.rs:109`
- [ ] **`Core\Heap::pop`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/heap.rs:127`
- [ ] **`Core\Heap::peek`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/heap.rs:118`
- [ ] **`Core\Heap::count`** and **`isEmpty`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/heap.rs:136`

## Backlog

- The perf figure for every feature this goal took — `python tools/dossier.py --record-perf`, once, at the goal's end.
- `Core\Hash\Stream` still holds every chunk until `finish` — `crates/nvs-stdlib/src/hash.rs`'s module doc § *`Hash\Stream` accumulates*.
