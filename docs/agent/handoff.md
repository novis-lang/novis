# Handoff

## State

Goal `core-env-and-4-more`, 8 of its 16 features taken. `Core\Env::all`, `get` and `mode`,
`Core\Fatal::onLimit` and `onUncaughtThrow`, and `Core\Hash::of`, `hmac` and `equals` each carry
`about.md`, three examples with blessed `.out` files, an attack, a bench declaring its counts and a
Rust `// covers:` test. The only proof they still owe is the perf figure. `python tools/dossier.py
--record-perf` writes it against a release build, and this goal records all of them in one commit
at its end.

`Core\Hash::of` and `hmac` now hold a digest inline (`crates/nvs-stdlib/src/hash.rs`'s `Octets`)
and allocate once per call; their benches declare `allocations 1`. `Core\Hash\Stream::finish`
still builds a `Vec` of the concatenated chunks, which the module doc's § *`Hash\Stream`
accumulates* owns.

Goal `limit-handler-reach`'s stage 3 still owns the `cpu_time` narrowed by `Core\Config::set`
that is accepted and never enforced.

## Next group

**`Core\Hash\Stream`'s three members** — one file set: `crates/nvs-stdlib/src/hash.rs`, plus the
proof trees under `core/Hash/stream`, `core/Hash/Stream/update` and `core/Hash/Stream/finish`
(`python tools/dossier.py --id` prints each path). `rule:testing/feature-proofs` owns what each
owes. The examples should agree with `Core\Hash::of` over the same input, so the reader can check
them. `update` after `finish` throws, and the attack should try it. The attack should also feed
many chunks, because the stream keeps every chunk until `finish`.

- [ ] **`Core\Hash::stream`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/hash.rs:337`
- [ ] **`Core\Hash\Stream::update`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/hash.rs:451`
- [ ] **`Core\Hash\Stream::finish`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/hash.rs:460`

## Backlog
- After `Hash\Stream`: `Core\Heap`'s five members, `crates/nvs-stdlib/src/heap.rs`. That is the goal's last group.
- At the goal's end: `python tools/dossier.py --record-perf` for all 16 features, in one commit.
- `python tools/dossier.py --bless` took over two minutes on nine files, most likely a release build.
  Start it in the background.
