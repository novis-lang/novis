# Handoff

## State

Goal `core-env-and-4-more`, 5 of its 16 features taken. `Core\Env::all`, `get` and `mode`, and
`Core\Fatal::onLimit` and `onUncaughtThrow`, each carry `about.md`, three examples with blessed
`.out` files, an attack, a bench declaring `allocations 0` and a Rust `// covers:` test. The only
proof they still owe is the perf figure. `python tools/dossier.py --record-perf` writes it against a
release build, and this goal records all of them in one commit at its end.

The `onLimit` proofs found one bug, scheduled rather than fixed: a `cpu_time` narrowed by
`Core\Config::set` is accepted and never enforced. It is goal `limit-handler-reach`'s new stage 3,
with a check naming the case that will pin it. `owners.py --check` refuses a goal as a gap's owner
and no open milestone's plan covers limits, so it is not a `# Known gaps` entry. No proof carries a
marker for it, because the examples end on the memory limit, which is enforced.

## Next group

**`Core\Hash`'s three static members** — one file set: `crates/nvs-stdlib/src/hash.rs`, plus the
proof trees under `core/Hash/`. `rule:testing/feature-proofs` owns what each owes, and the examples
should use a real digest the reader can check (`Core\Hash::of` over a known input). `equals` is
the constant-time comparison, so its attack should time nothing and compare very long and
empty inputs.

- [ ] **`Core\Hash::of`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/hash.rs:305`
- [ ] **`Core\Hash::hmac`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/hash.rs:314`
- [ ] **`Core\Hash::equals`** — `rule:testing/feature-proofs`; examples, hostile, perf, tests. `crates/nvs-stdlib/src/hash.rs:327`

## Backlog

- `Core\Hash::stream`, `Core\Hash\Stream::update` and `finish` — the group after this one, same file (`crates/nvs-stdlib/src/hash.rs:336`).
- `Core\Heap`'s five members — the last group of this goal (`crates/nvs-stdlib/src/heap.rs:109`).
- Every taken feature's perf figure — `python tools/dossier.py --record-perf` at the goal's end.
- The runtime-narrowed `cpu_time` that is never enforced — goal `limit-handler-reach`, stage 3 (`docs/agent/goals/172-limit-handler-reach.md`).
