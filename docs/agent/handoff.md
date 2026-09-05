# Handoff

## State

**Goal 6, M7 — ADR 0017's swap has landed, and ADR 0083 § 7's two `-p nvs-cli` names are green.**
`crates/nvs-cli/src/script.rs`'s `Compiler` is now § *Decision*'s five steps: a `PathEntry`
(digest, `mtime`/size stamp, `last_checked`) in front of a table keyed by
`nvs_config::cache::UnitKey`, resolved at `crates/nvs-cli/src/script.rs:235`. That module doc owns
what one core collapses — no compile pool, no `Compiling` state, no single-flight broadcast, and a
step 4 whose "unless a fresher revalidation won" is unreachable rather than relaxed — and what
survives: a `Failed` entry, so a request storm against a broken edit costs one compile.

**`[opcache] validate` and `revalidate_freq` are read**, into `nvs_config::cache::Revalidation`
(`crates/nvs-config/src/cache.rs:267`), whose defaults are decided at
`crates/nvs-config/src/cache.rs:244` — `mtime`, capped at 2s, PHP's own pair. `Compiler::new` takes
a `Config`; `nvs serve` and `nvs run` pass the booted snapshot's, `nvs test` keeps `default()`
because it resolves no tree on that path.

**Neither directive is refused at boot.** An unspelled `validate` falls back to the default;
`Validate::of` is where a check would read the word and `E0629` is free.

## Next group

**Stage 7's `nvs-server (hot reload)` check, which the swap unblocks. The file set is
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

- No boot refusal for `[opcache] validate` / `revalidate_freq` — `crates/nvs-config/src/cache.rs`'s
  module doc records it as owed; `nvs_config::log::validate` is the shape and `E0629` is free.
- `nvs test` compiles a `spawn script` under the default policy — `crates/nvs-cli/src/runner.rs:351`
  says why; closing it means resolving a tree on that path.
- The unit table keeps at most two entries per path, so a *reverted* edit recompiles rather than
  hitting — `crates/nvs-cli/src/script.rs:346`'s doc owns the trade.
- ADR 0042's on-disk cache is not consulted by the swap: a recompile is always a real compile.
  `crates/nvs-cli/src/cache.rs` holds the artifact store the new `UnitKey` could key into.
