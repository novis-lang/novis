# Handoff

## State

Goal `config-directives-3-3` covers fourteen `nvs.toml` directives. Seven are complete —
`log.target`, `metrics`, `mode.ceiling`, and this session's `mode.default`, `trace`, `opcache` and
`opcache.file_cache_dir` — and seven are owed: `[queue]`'s four keys, `schedule`, `server`,
`session`. Nothing is blocked. The acceptance check is `python tools/dossier.py --verify --only <the
fourteen>`, and `python tools/dossier.py --id 'directive:<key>'` says `complete.` for the seven.

A directive owes four artefacts and not five: `about.md`, one example with its blessed `.out`, one
attack, one `covers:`-marked case in `crates/nvs-config/tests/directives.rs`. No bench — the policy
excuses a directive from the perf proof.

`[context]` was widened this session rather than only described: `modules` now names `tree.rs`,
`export.rs`, `cache.rs` and `tests/directives.rs` beside the registry row, and `rules` names the
three config rules every slice asserts against. Both edits are in `docs/agent/loop-goal.toml` alone;
the generated goal file is what `--emit-goals` would rewrite, and it seeds `modules` from the
implementing anchor only (`tools/dossier.py:2799`).

## Next group

**Stage 2, the dossier — one file set:** `crates/nvs-config/src/directive.rs` for the row, a new
`docs/examples/config/<slug>/` and `tests/hostile/config/<slug>/` per key, and one case appended to
`crates/nvs-config/tests/directives.rs`. The `[queue]` block's four keys, which have no blanket row
over them and split two and two across `Apply` — that split is the registry case worth writing once
for the block rather than four times.

- [ ] **`directive:queue.connection`** — owes examples, hostile, tests. The connection the two job
      tables live in, `Boot` because every worker has dialled it
      (`rule:core-classes/queue-storage-is-a-table`, `rule:concurrency/enqueue-commits-with-your-write`).
      `crates/nvs-config/src/directive.rs:256`
- [ ] **`directive:queue.workers`** — owes examples, hostile, tests. How many job workers the serving
      process runs, `Boot` beside the connection
      (`rule:concurrency/one-process-serves-requests-schedules-and-jobs`).
      `crates/nvs-config/src/directive.rs:257`
- [ ] **`directive:queue.max_attempts`** — owes examples, hostile, tests. The `Reload` half of the
      block: read per job out of the snapshot
      (`rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`).
      `crates/nvs-config/src/directive.rs:261`
- [ ] **`directive:queue.visibility`** — owes examples, hostile, tests. How long a claimed job stays
      invisible, the second `Reload` key (`rule:core-classes/queue-storage-is-a-table`; the claim's
      own semantics are `nvs_stdlib::queue`'s module doc).
      `crates/nvs-config/src/directive.rs:262`

## Backlog

- `directive:schedule`, `directive:server` and `directive:session` are the goal's last three, each a
  whole block — `docs/agent/loop-goal.toml`'s acceptance check names them.
- `tools/dossier.py --emit-goals` seeds `[context] modules` from implementing anchors alone, so every
  directive goal opens with one file that says who may set a key and never what it does —
  `tools/dossier.py:2799`.
- An example under `docs/examples/config/` runs with no `nvs.toml`, so every `Core\Config::get` is
  `null` (`rule:config/no-configuration-file-is-a-complete-configuration`); a page's prose carries
  what the key resolves to instead.
