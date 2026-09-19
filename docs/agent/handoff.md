# Handoff

## State

Goal `lang:iteration` is live and three of its six features carry every feature proof:
`core-collections-are-iterable`, `generators` and `materialising-a-sequence-core-arr-from`.
`what-foreach-walks`, `the-two-interfaces` and `what-does-not-exist` owe all of theirs.

Two sentences in `docs/reference/lang/60-iteration.md` were wrong about the binary and are
rewritten: `current()` outside the protocol throws `LogicError` rather than answering a stale slot,
and `Core\Arr::from` accepts a `Core` collection. Both were already pinned by green conformance
cases.

The attack written against `Core\Arr::from` found a real hole: draining a sequence with no `limit`
grew past `[limits] memory` without bound, because the native loop passes no statement boundary and
calls no member, so nothing in it read the balance. One left running here reached 6 GB against a
256 MB ceiling. `crates/nvs-runtime/src/sequence.rs`'s `drain` now asks
`crate::abi::affordable` per element. The stop is pinned by
`tests/conformance/core/arr-from-over-a-sequence-with-no-end-is-stopped-by-the-memory-ceiling.nvst`.
That refusal does not run the program's `onLimit` handler, which `rule:errors/on-limit` says fires
for every resource `FATAL`; goal `limit-handler-reach` was inserted at the end of the chain for it.
Nothing is blocked.

## Next group

**Stage: feature proofs for `lang:iteration`** — one file set, the same one this session loaded:
`docs/reference/lang/60-iteration.md`, `docs/examples/lang/iteration/`,
`tests/hostile/lang/iteration/`, `benches/members/lang/iteration/`, `tests/conformance/iter/`.
One feature with all of its feature proofs is one slice, in the order
`rule:testing/feature-proofs` names them. The corpus under `tests/conformance/iter/` already holds
cases for all three; a `// covers:` marker in the `--FILE--` block is what attributes one, and two
per feature is what is owed.

- [ ] **`lang:iteration/what-foreach-walks`** — owes about, examples, hostile, perf, tests.
      `rule:iteration/foreach-subjects`. `docs/reference/lang/60-iteration.md:9`
- [ ] **`lang:iteration/the-two-interfaces`** — owes about, examples, hostile, perf, tests.
      `rule:iteration/two-interfaces`. `docs/reference/lang/60-iteration.md:29`
- [ ] **`lang:iteration/what-does-not-exist`** — owes about, examples, hostile, perf, tests. Every
      proof here is a refusal, so read `tests/hostile/README.md` on a compile diagnostic never
      being a pass before writing the attack. `docs/reference/lang/60-iteration.md:423`

## Backlog

- Editing a sentence of `docs/reference/lang/60-iteration.md` makes every `lang:iteration` perf
  figure stale; re-record the group before the gate (playbook, Tooling).
- `Core\Arr::from`'s own member feature still owes examples, a bench and a hostile case under
  `core/Arr/from/` — a different feature from this group's, and not this goal's.
- Goal `limit-handler-reach` (chain position 172) holds the handler half of the ceiling fix; its
  own handoff names the three slices.
