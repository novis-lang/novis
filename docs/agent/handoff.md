# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
stages 3 and 4 are complete. **Stage 5's surface is on disk**: `nvs service` names all seven verbs of
`rule:packaging/a-service-is-one-stored-argv`, each a `Site` over the applier `at_host`
(`crates/nvs-cli/src/service.rs:2680`) names, plus the hidden `nvs install-service` alias. An install
dry run, an uninstall and a control verb all run end to end against the real SCM on this machine.

**An uninstall reads back what it is undoing** — `Manager::stored`, which is `QueryServiceConfigW`'s
`ImagePath` and account on Windows and the unit's `ExecStart` on Linux. The two grant directories
neither manager holds are derived again from the configuration the stored argv names, so they are the
same paths the install granted; an install given the installer's own `--log-file` over a
configuration that names no `file:` destination is the single grant that derivation cannot reach, and
`service.rs`'s module doc owns that gap.

**`nvs service run` is the foreground spelling and nothing more.** `image_path` and `unit()` both
carry the stored argv directly, so a manager starts `nvs serve …` itself and never reaches this verb
— which puts `rule:packaging/a-service-answers-its-manager`'s table (STOP and PRESHUTDOWN to the
drain, PARAMCHANGE to the reload, the lifecycle records) inside `nvs serve` running under the SCM,
and that half is not on disk. Stage 5's `cargo-named` check is all that is left of the stage.
Nothing is blocked.

## Next group

**Stage 5: the cases, against the recording manager** — one file set: `crates/nvs-cli/src/service.rs`
(its `mod tests`) and `crates/nvs-cli/src/ctl.rs`.

- [ ] **The seven cases that need only the recording manager** — the three installs, the uninstall,
      `start`/`stop`/`status` by argv with no shell, and the refusal that runs before any manager
      call. `Recording` is `crates/nvs-cli/src/service.rs:2594`, and the front doors it is driven
      through are `crates/nvs-cli/src/service.rs:1711`, `crates/nvs-cli/src/service.rs:1731` and
      `crates/nvs-cli/src/service.rs:1747`; the exact names are the check at
      `docs/agent/loop-goal.toml:10201`. `rule:packaging/a-service-is-one-stored-argv`.
- [ ] **The SCM control handler belongs to `nvs serve`, and it is what the two `run` cases assert** —
      a control maps to a `State` (`crates/nvs-cli/src/service.rs:667`) through the `Supervisor` seam
      (`crates/nvs-cli/src/service.rs:712`), and `recording()`
      (`crates/nvs-cli/src/service.rs:791`) is the fake sink a case reads the checkpoints out of.
      The drain itself is one state machine entered at `Drain::process().begin()`.
      `rule:packaging/a-service-answers-its-manager`.
- [ ] **`service_status_reports_in_flight_and_drain_progress_from_the_control_socket`** — the verb is
      `crates/nvs-cli/src/service.rs:2838` and it already asks the socket named by the *stored*
      argv's configuration; the client half is `crates/nvs-cli/src/ctl.rs:75`.

## Backlog

- `crates/nvs-cli/src/service.rs` is now ~2,900 lines against [doc-style.md](doc-style.md)'s ~1,500
  target; the seam is `registration` and its two appliers, which is most of it.
- An install given `--log-file` over a configuration naming no `file:` destination leaves that one
  grant unrevoked at uninstall — `service.rs`'s module doc owns it, and binding the same value at
  `run` needs it recoverable too.
- `docs/agent/goals/23-per-core.md:87-89` still says `serve.rs` refuses `Listen::Unix` — retired
  goal's prose, owned by goal `plan-truth`.
- `rule:http-server/a-unix-socket-listener` is still `designed`; stage 13 flips it with `guardedBy`
  filled from stage 4's two cases.
- `cargo clippy --all-targets -- -D warnings` on the Linux leg fails on
  `crates/nvs-db/src/mysql.rs:325` and `pg.rs:375` (`large_enum_variant`); nothing gates it, because
  `verify.py` has no WSL leg.
- The unowned gaps at `crates/nvs-server/src/route.rs:30`, `bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
