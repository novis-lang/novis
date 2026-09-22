# Handoff

## State

Goal `core-env-and-4-more` is met. All 16 of its features — `Core\Env::all`, `get` and `mode`,
`Core\Fatal::onLimit` and `onUncaughtThrow`, `Core\Hash::of`, `hmac`, `equals` and `stream`,
`Core\Hash\Stream::update` and `finish`, and `Core\Heap::push`, `pop`, `peek`, `count` and
`isEmpty` — carry every feature proof, the perf figure included (`docs/perf/members.ndjson`).
The closing gates are green: `verify.py --doc`, `owners.py --closes core-env-and-4-more` and
`playbook.py --closes core-env-and-4-more` each name nothing, and `verify.py` is 14 of 14.

Goal `limit-handler-reach`'s stage 3 still owns the `cpu_time` narrowed by `Core\Config::set`
that is accepted and never enforced.

## Next group

**The next goal, `core-html-and-1-more`** — one file set: that goal's own `.md`, `.toml` and
`.handoff.md`, which the goal switch restores over this handoff.

- [ ] **Take the first feature of goal `core-html-and-1-more`** — `rule:testing/feature-proofs`;
      the goal file is `docs/agent/goals/dossier/113-core-html-and-1-more.md:1`, and its own
      handoff names the first slice.

## Backlog

- `Core\Env::all`'s bench measures 484 allocations per call, one per variable in the process
  environment; no bound is declared, so it is recorded, not judged — `crates/nvs-stdlib/src/env.rs`.
