# Handoff

## State

Goal `core-env-and-4-more`, all 16 of its features taken. `Core\Env::all`, `get` and `mode`,
`Core\Fatal::onLimit` and `onUncaughtThrow`, `Core\Hash::of`, `hmac`, `equals` and `stream`,
`Core\Hash\Stream::update` and `finish`, and `Core\Heap::push`, `pop`, `peek`, `count` and `isEmpty`
each carry `about.md`, three examples with blessed `.out` files, an attack, a bench and a Rust
`// covers:` test. The only proof they still owe is the perf figure, recorded in one commit at the
goal's end. The driver's acceptance failure (`target/release/nvs.exe is missing or older than the
tree`) is that release build not having been made yet, not a regression.

`Core\Heap`'s proofs found three bugs, all fixed and pinned. A callable reference
(`Class::method(...)`) whose target throws released its arguments twice, which corrupted the heap
(`tests/conformance/closures/a-callable-reference-that-throws-releases-each-argument-once.nvst`).
`Core\Heap::pop` leaked the element it took when the comparator threw. A comparator that takes
elements out of its own heap ended the request with a `FATAL`, and is now a catchable
`RuntimeError`. The heap also reaches its entries by integer key, so `push` allocates nothing.

Goal `limit-handler-reach`'s stage 3 still owns the `cpu_time` narrowed by `Core\Config::set`
that is accepted and never enforced.

## Next group

**The goal's end: perf figures and the closing gates** — one file set: `docs/perf/members.ndjson`
and the release build `tools/dossier.py` makes for itself.

- [ ] **Record the perf figure for all 16 features** — `rule:testing/member-perf-ledger`; `python
      tools/dossier.py --record-perf` builds `target/release/nvs.exe` itself first (about six
      minutes, `tools/dossier.py:513`), then measures every stale feature. Commit the ledger lines
      in one commit.
- [ ] **Close the goal** — `rule:testing/feature-proofs`; `python tools/verify.py --doc`, then
      `python tools/owners.py --closes core-env-and-4-more` and `python tools/playbook.py --closes
      core-env-and-4-more`, then `DONE` in `.loop/status.txt`. `docs/agent/loop-goal.md:1`

## Backlog

- `fn (string $a): int => throw new LogicError('no')` does not compile: `E0401 expected int, found
  never`. A `never` expression should be accepted where any type is expected. Owner: `nvs-types`'
  module doc `# Known gaps` (not checked whether it already names this).
