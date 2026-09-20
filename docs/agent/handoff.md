# Handoff

## State

Goal `core-bytes` is met. All thirteen `Core\Bytes` members carry the feature proofs
`rule:testing/feature-proofs` makes them owe: `python tools/dossier.py --gate --group 'Core\Bytes'`
reports nothing owed, its 39 examples and 13 attacks all pass, `python tools/verify.py` is 14 of 14
green and `python tools/verify.py --doc` resolves every link. `python tools/owners.py --closes
core-bytes` and `python tools/playbook.py --closes core-bytes` each name nothing.

`pack` and `unpack` were the last two, and they read one format grammar between them. Their figures
are 178.4 ns/op with 7 allocations for `pack` and 102.8 with 3 for `unpack`.

No proof found a bug: both members answered what their reference cards say, at both ends of their
ranges, and every step of both attacks stopped where the card says it should. One stale claim went:
`mixed as bytes` lowers now, so the conformance case that said it does not was rewritten.

## Next group

The chain's next goal is `core-cache-and-5-more`, whose own handoff stub
(`docs/agent/goals/dossier/99-core-cache-and-5-more.handoff.md`) is installed over this file at the
switch. Its first group is three members of one file set — `crates/nvs-stdlib/src/cache.rs`,
`docs/examples/core/Cache/`, `tests/hostile/core/Cache/`, `benches/members/core/Cache/` — each
owing all of `rule:testing/feature-proofs`:

- [ ] **`Core\Cache::local`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/cache.rs:188`
- [ ] **`Core\Cache::process`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/cache.rs:197`
- [ ] **`Core\Cache::shared`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/cache.rs:206`

## Backlog

- `Core\Bytes::pack` costs 7 allocations and 333 bytes per call for a five-byte result, most of it
  the variadic argument array a call site builds — the figure is in `docs/perf/members.ndjson`.
- `pack` and `unpack` still owe the two classifications their own doc comment names, an intrinsic
  literal format and a sink qualifier: `crates/nvs-stdlib/src/bytes.rs:1449`.
