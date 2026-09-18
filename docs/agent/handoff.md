# Handoff

## State

Goal `config-directives-3-3` covers fourteen `nvs.toml` directives; three of them were taken this
session and eleven are owed. Nothing is blocked, and the acceptance check is
`python tools/dossier.py --verify --only <the fourteen>`.

On disk now: a page, one example with its blessed `.out` and one attack for `directive:log.target`,
`directive:metrics` and `directive:mode.ceiling`, plus one registry case each in
`crates/nvs-config/tests/directives.rs`. `python tools/dossier.py --id 'directive:log.target'` says
`complete.` for all three.

Two gaps the pack paid for twice this session: `[context] modules` names only
`crates/nvs-config/src/directive.rs`, so the semantics of a block have to be fetched by hand from
`log.rs`, `export.rs` and `tree.rs` in the same crate; and `[context] rules` names no rule that
states what a directive *does*, so `rule:http-server/the-mode-ceiling-defaults-to-the-startup-mode`
and `rule:errors/engine-floor` were both read on their own call.

## Next group

One slice is one feature with all four proofs. The file set is the same one this session used —
`crates/nvs-config/src/directive.rs` for the row, the crate's own validator beside it, a new
`docs/examples/config/<slug>/` and `tests/hostile/config/<slug>/`, and one case appended to
`crates/nvs-config/tests/directives.rs`. The first two are the other halves of blocks this session
already read; the last two are one block and are cheapest taken together:

- [ ] **`directive:mode.default`** — owes examples, hostile, tests. The `Runtime` half of the block
      `directive:mode.ceiling` bounds (`rule:config/a-program-may-read-and-flip-its-mode`).
      `crates/nvs-config/src/directive.rs:114`
- [ ] **`directive:trace`** — owes examples, hostile, tests. The push-only block beside `[metrics]`,
      same validator and same refusal (`rule:observability/metrics-and-trace-blocks-are-system`);
      its `sample` is the fraction `E0628` refuses. `crates/nvs-config/src/directive.rs:266`
- [ ] **`directive:opcache`** — owes examples, hostile, tests.
      `rule:config/opcache-file-cache-directives-are-system`. `crates/nvs-config/src/directive.rs:230`
- [ ] **`directive:opcache.file_cache_dir`** — owes examples, hostile, tests. The one `[opcache]` key
      that is `Boot` rather than `Reload`. `crates/nvs-config/src/directive.rs:237`

## Backlog

- Seven features of this goal are untouched after the group above: `server`, `session`, `schedule`
  and the four `queue.*` — `python tools/dossier.py --owed --only <the fourteen>` is the list.
- No directive in this goal owes a bench; `tools/data/dossier-policy.toml` is where that is written.
