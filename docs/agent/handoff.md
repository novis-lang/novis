# Handoff

## State

**Goal `plan-truth` — every plan file and module doc says what the tree does — is through stages 2 and 3.**
`python tools/plan.py --stale` exists and answers `sentences deferring to a walked goal: 0`, which is the
stage-2 acceptance check; every milestone file `docs/plan/m1.md` through `m8.md`, plus the index's status
block and its schedule paragraph, now state what is on disk rather than what a walked goal would do.

What that changed, and what it rests on: M1 is whole (the pipeline operator at
`crates/nvs-syntax/src/token.rs:271`, the PHP 8.6 refusals at `:487`/`:491`, `===`/`!==` reported at
`crates/nvs-syntax/src/lexer.rs:1039`); M4's corpus figure is met; M4S's spec ratchet holds no keys; M5's
`Task::all` callable-variable refusal is gone (`crates/nvs-types/tests/core_members.rs:407`); the M6 control
socket, M7's body surface and M5's `spawn` trace event name goals `m7-server-surface` and `m5-proofs`, which
have not walked. Stages 1, 4 and 5 are untouched — stage 1 is goal `websocket-client`'s list, carried as the
floor.

## Next group

**Stage 4: the module docs whose gaps the code already closed** — one file set: the three crate `//!`
headers the goal's `[context] modules` already names.

- [ ] **`nvs serve` reads the config and takes every core** — `crates/nvs-cli/src/serve.rs:79` strike the
      "a served request carries no configuration" gap (M6-tagged) with `crates/nvs-server/src/serve.rs:1101`
      beside it, and rewrite any one-core wording against `crates/nvs-cli/src/serve.rs:427`.
      `rule:security/isolate-shares-nothing` is the rule the header sits under.
- [ ] **`|>` is a token** — `crates/nvs-syntax/src/lib.rs:57` says it is not one and must not become one;
      `crates/nvs-syntax/src/token.rs:271` and `rule:expressions/pipeline-substitution` are what it is now.
- [ ] **`nvs.toml` is read** — `crates/nvs-test/src/lib.rs:150` says it is not until M6, which has walked.
      Say what `--INI--` does now, or record why it stays refused
      (`rule:config/the-file-is-nvs-toml-and-it-is-toml`).

## Backlog

- `§16 Core\Metrics` is the one key left in `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`
  and no goal registers it — stage 5's index, `docs/agent/carried-gaps.md` § *Unowned*.
- `crates/nvs-runtime/tests/perf_guards.rs:1118` still says `PropertyObserver` has no implementation
  (`crates/nvs-stdlib/src/interfaces.rs:48` registers it) — stage 4, a different file set.
- The `nvs-ir` half of `rule:expressions/one-equality-operator` (the null tag test and the three helpers) is
  `docs/plan/m2.md:42`'s deferral to `nvs-ir`'s own gap list; not re-checked this session.
- `[context] modules` names no `docs/plan/` path, so stage 3's own file set was not in the pack; harmless
  now that the stage is done, and worth adding if a later stage reopens a milestone file.
