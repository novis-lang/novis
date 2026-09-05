# Handoff

## State

**Goal 6, M7 — stage 7's `nvs-cli (nvs service, and it fails closed)` check is green.** All seven of
ADR 0093's names run and pass under `cargo test -p nvs-cli`. The driver's next failure will be the
`nvs-server (hot reload)` check, now at `docs/agent/loop-goal.toml:3915` (this session's manifest
edit moved it by ten lines), which is the group below.

**The whole check was writable with no privileged operation, because § 2 is a pure function.**
`crates/nvs-cli/src/service.rs` is the installer sink: `plan()` is the one constructor of a `Plan`
and the only path out of it that is not one of `E0630`-`E0634`, and `Host` is a *parameter* — what
this binary and the named config answer about themselves — so every refusal is reachable with no
service manager, no elevation and no files. `image_path` is § 3's `CommandLineToArgvW` encoder with
`decode` beside it so the round trip is a property; `unit` is § 5's systemd unit; `destination`
answers `None` for the printing delivery, which is what makes "written only on install" assertable.

**What is not there is registration**, and the module doc says so: no `install`, `uninstall`,
`start`, `stop`, `status` or `run` subcommand, no SCM call and no unit-directory write. That is the
goal's own standing decision ("No session installs a service") plus ADR 0093's *Verification*, whose
remaining items all need a privileged machine. `nvs service unit` is the one wired subcommand.

**`orient.py` did not print ADR 0093** — that gap is now closed differently than the last two
sessions closed it: `[context] adrs` gained `0017 Decision` and `0091 §3a` for the group below, and
`docs/agent/goals/6-server.toml` was re-synced from the live file.

## Next group

**Stage 7's `nvs-server (hot reload)` check. The file set is `docs/agent/loop-goal.toml`,
`crates/nvs-cli/src/script.rs` and `crates/nvs-config/src/cache.rs`.** All three names are misfiled
`-p nvs-server`: two are the CLI cache's and one is `nvs_config::cache`'s.

- [ ] **Refile the check and write `revalidation_is_lazy_and_rate_capped`** — split
      `docs/agent/loop-goal.toml:3915` so the two cache names run `-p nvs-cli`, and assert both
      halves of ADR 0017 § *Decision* step 1 over `crates/nvs-cli/src/script.rs:235`:
      `validate = "never"` never `stat`s, and two resolves inside one `revalidate_freq` window make
      one check while one past it makes two. Count the checks by editing the file between resolves
      and reading which unit comes back — `crates/nvs-cli/src/script.rs:397` is the only place a
      syscall happens, and `revalidating()` at `crates/nvs-cli/src/script.rs:552` is the fixture.
- [ ] **`a_swap_never_blocks_a_request_serving_core`** — ADR 0017 § *Decision*'s paragraph after the
      five steps, over `crates/nvs-cli/src/script.rs:235`: a resolve that finds a newer source
      publishes the new unit without any in-flight resolve waiting on it.
- [ ] **`the_validate_default_is_selected_by_the_run_mode`** — and **it is not `nvs_config::mode`'s**,
      which the previous handoff guessed. `crates/nvs-config/src/mode.rs:8` says outright that
      § 3a's three startup rows are deliberately *not* in that module's table. The default lives at
      `crates/nvs-config/src/cache.rs:204` (`Validate`'s `#[default]`, whose doc already cites
      ADR 0091) and is read at `crates/nvs-config/src/cache.rs:262` (`Revalidation::of`), which
      today reads `[opcache]` and does **not** take a mode — so check whether the mode→`validate`
      link exists before writing the test.

## Backlog

- Registration itself — `install`/`uninstall`/`start`/`stop`/`status`/`run`, the SCM call and the
  systemd write. `crates/nvs-cli/src/service.rs`'s module doc § *What is on disk, and what is not*.
- `nvs ctl` has no client and § 3's `ctl config` is not an operation —
  `crates/nvs-server/src/control.rs`'s module doc.
- Nothing accepts on the control endpoint: no `hyper` connection is served over it and `nvs serve`
  creates none. Same module doc.
- `--fault-inject` is matched by word in `service.rs`; if `nvs run` ever gains a second hook the
  allowlist there needs the same treatment. ADR 0093 § 2, row 2.
- `[server] listen`'s privileged-port test in `service.rs::describe_host` reads the merged table
  rather than `nvs_config::server`'s typed value. ADR 0097 § 5.
- `Core\Session` may not use the local cache tier — ADR 0059 § 4, still enforced by boot refusal
  only.
