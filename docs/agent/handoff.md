# Handoff

## State

**Goal 6, M7 — stage 7's `nvs-server (the control socket)` check is green.** All five of ADR 0078's
names run and pass under `cargo test -p nvs-server`; the driver's next failure will be the
`nvs-server (hot reload)` check at `docs/agent/loop-goal.toml:3905`, which is the group below and
was this session's original item.

**The surface is split across two crates, and the split is the point.**
`crates/nvs-config/src/control.rs` owns the endpoint, because "no account but this one may reach
it" is a mode on Unix and a DACL on Windows and `nvs_config::trust` already owns both spellings:
`Address::of` reads `[control] socket` (absent or `false` is `Disabled`, and `E0629` refuses
anything network-shaped), `Endpoint::create` makes an `AF_UNIX` socket at mode `0600` or a named
pipe under a `D:P` DACL naming this account, `SYSTEM` and `Administrators`, `boundary`/`bind` put
§ 3's directory rule in front of it, and `reload` publishes a snapshot and returns § 5's `Report`.
`crates/nvs-server/src/control.rs` owns the wire surface — `Operation::of`, which is `POST /reload`
and nothing else — and re-exports the rest so the server's control surface is one name.

**What is not there is the transport.** Nothing accepts on the endpoint, no `hyper` connection is
served over it, `nvs serve` does not create one, and `nvs ctl` has no client. § 3's `ctl config`
(ADR 0103 § 9) is deliberately not an operation yet and `control.rs`'s module doc says so.

**`orient.py` did not print ADR 0078** — the `[context] adrs` list has no entry for it, so §§ 3, 5
and 6 were sliced by hand. Add `0078` with those three sections.

## Next group

**Stage 7's `nvs-server (hot reload)` check, which ADR 0017's landed swap unblocks. The file set is
`docs/agent/loop-goal.toml`, `crates/nvs-cli/src/script.rs` and `crates/nvs-config/src/cache.rs`.**
All three of its names are misfiled `-p nvs-server`: two are this cache's and one is
`nvs_config::mode`'s, and the check is at `docs/agent/loop-goal.toml:3905`.

- [ ] **Refile the check and write `revalidation_is_lazy_and_rate_capped`** — split
      `docs/agent/loop-goal.toml:3905` so the two cache names run `-p nvs-cli`, and assert both
      halves of step 1 over `crates/nvs-cli/src/script.rs:235`: `validate = "never"` never
      `stat`s, and two resolves inside one `revalidate_freq` window make one check while one past
      it makes two. Count the checks by editing the file between resolves and reading which unit
      comes back — `crates/nvs-cli/src/script.rs:397` is the only place a syscall happens, and
      `revalidating()` in that module's tests is the fixture shape.
- [ ] **`a_swap_never_blocks_a_request_serving_core`** — ADR 0017 § *Decision*'s paragraph after
      the five steps. On one core the claim is that no borrow of this cache is held across a
      compile: a program resolved from it, *while running*, does a `spawn script` over a second
      path, which compiles through the same `Compiler` (`crates/nvs-cli/src/script.rs:346` is the
      only `borrow_mut` a compile is anywhere near). `examples/isolate/` has the spawning fixtures;
      a held borrow is a panic rather than a wrong answer, which is what makes this assertable.
- [ ] **`the_validate_default_is_selected_by_the_run_mode`** — ADR 0091 § 3a, and it is
      `nvs-config`'s: `production` starts at `Validate::Never`, `development` at `Mtime`.
      `crates/nvs-config/src/mode.rs:8` says § 3a's three startup rows are deliberately not in that
      module, and `crates/nvs-config/src/tree.rs:773` carries the other two on their own fields, so
      this row goes where it is read — `Revalidation::from_config`,
      `crates/nvs-config/src/cache.rs:267`, which needs the snapshot's `mode` beside the `Config`.
      Move the name to `-p nvs-config` in the same `loop-goal.toml` edit as the first item.

## Backlog

- The control endpoint has no accept loop, no `hyper` connection over it and no `nvs ctl` client —
  `crates/nvs-config/src/control.rs`'s module doc owns what exists; ADR 0078 § 3 owns the rest.
- No boot refusal for `[opcache] validate` / `revalidate_freq` — `crates/nvs-config/src/cache.rs`'s
  module doc records it as owed; `nvs_config::log::validate` is the shape and `E0630` is now free.
- `nvs test` compiles a `spawn script` under the default policy — `crates/nvs-cli/src/runner.rs:351`
  says why; closing it means resolving a tree on that path.
- The unit table keeps at most two entries per path, so a *reverted* edit recompiles rather than
  hitting — `crates/nvs-cli/src/script.rs:346`'s doc owns the trade.
- ADR 0042's on-disk cache is not consulted by the swap: a recompile is always a real compile.
  `crates/nvs-cli/src/cache.rs` holds the artifact store the new `UnitKey` could key into.
