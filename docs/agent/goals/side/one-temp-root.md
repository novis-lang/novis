# Side goal — nothing writes into the shared temp directory except Novis's own temp root

When this goal is green, one function in the whole tree reads the system temp directory:
`nvs_runtime::capability::temp_root`, which gives Novis's own temp root. Every test writes its scratch
under `target/`, Unix sockets included, and deletes it when it ends. `nvs lsp-test` and `nvs test`'s
`.nvst` runner create their working directory under the temp root, privately and under a name the
orphan sweep knows. A guard test fails on any new call, in test code and product code alike.

## Why a side goal

Side goal `test-scratch-and-hover` moved most test scratch under `target/` and landed on 2026-10-05
with the rest written down as two gap records. The user asked for the rest to be fixed properly, as one
side goal, so the chain run is not disturbed. Nothing on the chain waits for it. It touches test
modules in many crates, the two runners and one function in `nvs-runtime`.

## What is on disk today, measured

- **The guard and its list.** `no_test_code_writes_into_the_system_temp_dir` in
  `crates/nvs-repo/tests/scratch.rs` reads test code only. Its `NOT_YET_MOVED` list names what has not
  moved: files in `nvs-stdlib` (16), `nvs-db` (2), `nvs-host` (2), `nvs-footprint`, `nvs-test`,
  `nvs-config/tests/control.rs` and `nvs-server/src/serve.rs`. `SUBJECT` is empty.
- **Eight of those sites bind a Unix socket:** `crates/nvs-config/tests/control.rs:49`,
  `crates/nvs-server/src/serve.rs:8086`, `crates/nvs-host/src/net.rs:2662`, `crates/nvs-db/src/pg.rs:4080`,
  `crates/nvs-db/src/mysql.rs:5247` and `:5351`, `crates/nvs-stdlib/src/cache/redis.rs:885`,
  `crates/nvs-stdlib/src/cache.rs:4569` and `crates/nvs-stdlib/src/net.rs:2428`. A socket path is limited
  to 104 bytes on macOS and 108 on Linux, and the bind fails with `InvalidInput` past that.
- **Three hand-written helpers already put sockets under `target/`:** `crates/nvs-cli/src/ctl.rs:460`,
  `crates/nvs-cli/src/serve.rs:3894` and `crates/nvs-server/src/control.rs:489`, each in a folder beside
  the test binary. Their doc comments give the reason: `rule:config/ownership-is-the-trust-boundary`
  also checks the parent folder, and `/tmp` is world-writable. They pass on the macOS CI legs
  (`.github/workflows/ci.yml:130`). They never delete their folder.
- **The two runners.** `crates/nvs-lsp/src/suite.rs:339` (`Materialised::write`) and
  `crates/nvs-test/src/lib.rs:353` create `<system temp>/nvs-lspt-<pid>-<n>` and
  `<system temp>/nvs-test-<pid>`: they delete the name, then create it. The name can be guessed, so on a
  shared Unix machine another account can create that folder first. Not checked: whether that can be
  exploited end to end. Neither runner resolves a configuration today (`crates/nvs-cli/src/main.rs:1754`
  and `:3447`).
- **The runtime's own temp root.** `capability::temp_root` (`crates/nvs-runtime/src/capability.rs:1273`)
  is `[io] temp_root`, else `<system temp>/novis`. `capability::temp_dir` (`:1204`) creates
  `nvs-<pid>-<16 hex digits>` under it, atomically and owner-only, but needs a `Ctx` and a grant.
  `sweep::orphans` (`crates/nvs-runtime/src/sweep.rs:251`) deletes such a folder once its process is
  gone, from the `nvs serve` boot and `nvs tmp clean`.
- **`[io] temp_root` and relative paths.** `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`
  covers every path-valued directive, but its list does not name `io.temp_root`. Not checked: whether a
  relative `temp_root` resolves against its file today. The repository's `nvs.toml` has no `[io]` table.
- **A test that leaks a folder nobody can delete.**
  `the_icacls_steps_turn_a_folder_every_account_may_change_into_one_nvs_init_writes_into`
  (`crates/nvs-cli/tests/init.rs:466`) removes this account's rights from its scratch folder. Its
  `remove_dir_all` at the end then fails, and so does the `Scratch` guard. On 2026-10-05 two such
  folders were left in a side worktree and needed `icacls /grant` before they could be deleted.

## Stage 1 — the floor

Main's carried floor, which a side run is always checked against. Never traded.

## Stage 2 — every test writes under `target/`, sockets included

**Does:** Adds a socket helper to `nvs-repo`, moves every site left in `NOT_YET_MOVED` to `nvs_repo::scratch` or the new helper, and fixes the test that leaks a locked folder.

File set: `crates/nvs-repo/src/lib.rs`, `crates/nvs-repo/tests/scratch.rs`, the `Cargo.toml` of each
crate that gains `nvs-repo` as a dev-dependency, and the test modules the list names.

- **The socket helper** is `nvs_repo::socket(name: &str) -> (Scratch, PathBuf)`. It makes a scratch
  folder with a short name, such as `target/test-scratch/sock-<pid>-<n>/`, and returns the guard and
  the socket's absolute path in it. A path longer than the platform allows stops the test with a
  message that names the limit and says to set a shorter `CARGO_TARGET_DIR`. A relative path is not
  used: the ownership check walks parent folders, and a relative path cuts that walk short.
- **Every socket site moves** to it, and so do the three hand-written helpers (`ctl.rs:460`,
  `serve.rs:3894`, `control.rs:489`), which then delete their folder too.
- **Every other site moves** to `nvs_repo::scratch`. A test that runs the ownership check uses
  `nvs_repo::scratch_private`. Each row of `NOT_YET_MOVED` is lowered or removed in the commit that
  moves its site, and the stage ends with the list gone.
- **The `icacls` test gives its rights back.** It restores this account's rights to its folder before
  the folder is deleted, and does that from a guard's `Drop`, so a failing run also leaves nothing.
- **Gap record** `data/gaps/nvs-repo/some-test-code-still-writes-into-the-system-temp-dir.json` is
  deleted in the commit that empties the list.

## Stage 3 — the two runners use Novis's own temp root

**Does:** Lets code without a script context create a private folder under the temp root, makes `nvs lsp-test` and the `.nvst` runner use it, and extends the guard to product code.

File set: `crates/nvs-runtime/src/capability.rs`, `crates/nvs-lsp/src/suite.rs`,
`crates/nvs-test/src/lib.rs`, `crates/nvs-test/src/run.rs`, `crates/nvs-cli/src/main.rs`,
`crates/nvs-repo/tests/scratch.rs`, `nvs.toml`.

- **A private folder without a `Ctx`.** The create loop in `capability::temp_dir` moves into a public
  function that takes the root and returns a new `nvs-<pid>-<nonce>` folder, created atomically and
  owner-only. `temp_dir` keeps its grant check and its tracking, and calls it.
- **The runners take a root.** `nvs_lsp::suite::run` and `nvs_test::run` (through `Options`) take the
  folder to work under from their caller and delete it when they finish. A run that is killed leaves a
  folder whose name the orphan sweep knows. Tests that call the runners in-process pass a
  `nvs_repo::scratch` folder.
- **The CLI passes `temp_root`.** `nvs lsp-test` and the `.nvst` branch of `nvs test` resolve the
  configuration tree the way `nvs run` does, never creating `nvs.toml`, and pass
  `capability::temp_root` of it.
- **This repository's runs stay in the tree.** The repository's `nvs.toml` sets `[io] temp_root` to a
  folder under `target/`. If a relative `temp_root` does not resolve against its file today, make it
  do so, as the relative-path rule already says for every path-valued directive, and add `io.temp_root`
  to that rule's list.
- **The guard covers everything.** `nothing_but_the_temp_root_reads_the_system_temp_dir` replaces
  `no_test_code_writes_into_the_system_temp_dir`. It reads all code under `crates/`, test code and
  product code, and allows `capability::temp_root` and the tests in `SUBJECT`. It has no list of sites
  waiting to move.
- **Gap record** `data/gaps/nvs-lsp/lsp-test-writes-its-cases-into-the-system-temp-dir.json` is deleted.
- **The rules.** Amend `rule:core-classes/temporary-dir-sweep` to say the two runners create their
  folders under the temp root too. Also amend the `nvs lsp-test` page,
  `docs/examples/tools/editor/nvs-lsp-test/about.md`, if it says where cases are written.

## Standing decisions

These are the user's calls, made on 2026-10-05, unless marked as mine. No session re-decides one.

- **Fix both properly, as one side goal.** The design in stages 2 and 3 was my recommendation on
  2026-10-05, and the user asked for this goal on it.
- **Mine, 2026-10-05:** sockets go under `target/test-scratch/` with an absolute path, never a relative
  one; the helper stops with the limit named; the three hand-written socket helpers fold into it.
- **Mine, 2026-10-05:** the runners use the runtime's own temp root, `[io] temp_root`, with no new flag
  or setting; the library takes a root and the CLI decides it; the repository's `nvs.toml` points it
  under `target/`.
- **ADR slots: none.** Amend a rule fragment where it states the old behaviour.
- **The tradeoffs, stated once.** Performance and memory: none. Security: the runners stop using a
  guessable name in a shared folder. Users: `nvs lsp-test` and `nvs test` now write where
  `[io] temp_root` says, the same place as every other Novis temp folder, and a killed run's folder is
  removed by `nvs tmp clean`. Contributors: one helper for scratch, one for sockets, and a guard with no
  list to maintain.
