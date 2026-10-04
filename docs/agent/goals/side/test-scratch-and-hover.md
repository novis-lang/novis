# Side goal — test scratch stays inside the tree, and hover answers on every class name

When this goal is green, no test writes into the system temp directory: every test that needs a
scratch directory gets one under `target/` from one helper, and the directory is gone when the test
ends. A guard test fails on any new `std::env::temp_dir()` in test code. And in the editor, hovering a
class name shows its card wherever go-to-definition already jumps: on a `use` line, in an `implements`
clause and in a type position. *Find References* lists the `use` lines that import a class, and the
CodeLens count stays the count of real uses.

## Why a side goal

The user decided both on 2026-10-04 and asked for them as one side goal, so the chain run is not
disturbed. Nothing on the chain waits for either. The first part touches test code in twelve crates and
the second touches `crates/nvs-lsp` alone, so neither opens a file the chain's live goal works in, apart
from rebase noise in test modules.

## What is on disk today, measured

- **The leak.** Test code calls `std::env::temp_dir()` at 97 sites in 12 crates: `nvs-cli` 30, `nvs-lsp`
  26, `nvs-stdlib` 18, `nvs-runtime` 7, `nvs-test` 3, `nvs-host` 3, `nvs-db` 3, `nvs-server` 2,
  `nvs-config` 2, `nvs-types` 1, `nvs-hir` 1, `nvs-footprint` 1. Typical shapes:
  `crates/nvs-cli/src/script.rs:1962` (`nvs-swap-<pid>-<case>`), `crates/nvs-cli/src/cache.rs:1816`
  (`nvs-cache-<pid>`), `crates/nvs-cli/src/config.rs:744` (`nvs-init-<pid>`),
  `crates/nvs-cli/tests/init.rs:23` (`nvs-init-cmd-<pid>`). Many never delete what they made. On the
  user's machine `%TEMP%` held 59,638 entries on 2026-10-04, of them about 57k `nvs-*` test directories
  (308 MB) added at about 3,000 a day since 2026-09-22. They were deleted by hand that day.
- **Not part of the leak.** Three product uses of the system temp directory stay as they are:
  `crates/nvs-runtime/src/capability.rs:1220` (the runtime's own temp root, `%TEMP%\novis` unless
  `[io] temp_root` is written), `crates/nvs-test/src/lib.rs:353` (`nvs test`'s scratch, removed at the
  start of the next run) and `crates/nvs-lsp/src/suite.rs:339` (`nvs lsp-test`'s materialised cases).
- **The boot sweep is not affected.** `serve::sweep_orphans` lists only the runtime's temp root
  (`capability.rs:1220`), which held 25 entries. The leftover test directories sit beside it.
- **Where a helper goes.** `crates/nvs-repo` is already how a test reaches a path outside its package
  (its module doc), and six of the twelve crates already have it as a dev-dependency (`nvs-cli`,
  `nvs-lsp`, `nvs-stdlib`, `nvs-runtime`, `nvs-config`, `nvs-types`).
- **Hover.** `crates/nvs-lsp/src/hover.rs:121-140` answers from expression nodes, attribute payloads,
  autoload prefixes, written class names, path arguments and completion-file values. It never asks
  `definition::type_name_at` (`crates/nvs-lsp/src/definition.rs:1127`) or `definition::import_at`
  (`definition.rs:828`), which is why go-to-definition jumps from a `use` line, an `implements` clause
  and a type position while hover shows nothing there.
- **References and the lens.** `server.rs:871` `references` answers from the index's occurrences. The
  index keeps a file's `use` lines apart, as `Import` (`crates/nvs-lsp/src/index.rs:273`), so they are
  not occurrences. The CodeLens counts `index.occurrences(...)` (`server.rs:1165`). There is no rename
  provider (`capabilities.rs:153`).

## Stage 1 — the floor

Main's carried floor, which a side run is always checked against. Never traded.

## Stage 2 — test scratch under `target/`

**Does:** Gives every test one scratch helper under `target/`, moves all 97 sites to it, and adds a
guard test that fails on a new one.

File set: `crates/nvs-repo/src/lib.rs`, `crates/nvs-repo/tests/scratch.rs` (new), the `Cargo.toml` of
each crate that gains the dev-dependency, and the test modules of the 97 sites.

- **The helper** is `nvs_repo::scratch(name: &str) -> Scratch`. It creates
  `<repository>/target/test-scratch/<name>-<pid>-<n>` and returns a guard whose `path()` is that
  directory. Dropping the guard deletes the directory, also when the test fails.
- **It records nothing.** A scratch directory is written, never read from the tree, so it is not a
  `path` or `named` read and never enters `NVS_READS_LOG` or a footprint. `tools/nv/keys/escape.ts`
  must accept the call. A test that only uses scratch stays narrow.
- **Every site moves** to the helper. A test that kept its directory across two steps keeps the guard
  alive for both. A test whose subject is the system temp directory itself (a Windows 8.3 alias, the
  runtime's default temp root) keeps `std::env::temp_dir()`, and the guard lists it with its reason.
- **The guard test** `no_test_code_writes_into_the_system_temp_dir` in `crates/nvs-repo/tests/scratch.rs`
  reads every `.rs` file under `crates/*/tests/` and every `#[cfg(test)]` module under `crates/*/src/`,
  and fails on `std::env::temp_dir()` or `env::temp_dir()` outside its own list. It records what it reads
  through `nvs_repo::path`.
- **The helper's own tests:** `a_scratch_dir_is_under_the_target_dir` and
  `a_scratch_dir_is_removed_when_its_guard_drops`.

## Stage 3 — hover on every class name, and `use` lines in references

**Does:** Makes hover answer wherever go-to-definition jumps to a class, and lists `use` lines in
*Find References* without counting them in the CodeLens.

File set: `crates/nvs-lsp/src/hover.rs`, `crates/nvs-lsp/src/definition.rs`,
`crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/index.rs`, `tests/lsp/hover/`,
`tests/lsp/references/`, `tests/lsp/lens/`.

- **Hover** asks `definition::type_name_at` and then `definition::import_at` as its last fallback, and
  renders the same card a class name in an expression gets. The underlined range is the name.
- **References** add every `use` line whose import names the symbol, from the index's `Import` rows.
  The CodeLens count stays `occurrences` only, so an import does not look like a use.
- **Cases**, one per position: a `use` line, an `implements` clause, a parameter type, a return type and
  an `array<T>` argument for hover; a `use` line listed by references; a lens that does not count it.
  A case that reaches a construct no case reached before makes the coverage matrix ask for a case for
  every other request. Write those cases, or keep that position under a unit test instead.
- **The rules.** No fragment under `docs/rules/ide/` lists where hover answers today; the nearest is
  `ide/the-index-answers-the-cursor`. Amend any fragment that states otherwise.

## Standing decisions

These are the user's calls, made on 2026-10-04, unless marked as mine. No session re-decides one.

- **No test writes into the system temp directory.** The scratch directory is under `target/` and is
  deleted when the test ends. A guard test keeps it that way. This is AGENTS.md rule 10 applied to tests.
- **The boot sweep stays where it is**, before the socket is bound.
- **Hover answers on a `use` line, an `implements` clause and a type position.**
- **A `use` line is listed by *Find References* and not counted by the CodeLens.**
- **One side goal for both**, so the chain run is not disturbed.
- **Mine, 2026-10-04:** the helper lives in `nvs-repo` and records no read; the directory is
  `target/test-scratch/`; it is deleted on failure too; the guard reads source as text; the names of the
  helper's tests.
- **ADR slots: none.** Amend a rule fragment where it states the old behaviour.
- **The tradeoffs, stated once.** Runtime performance and memory: none, because nothing here changes
  `nvs` or the server. Tests write their scratch under `target/`, inside the project, and delete it, so
  the user's temp directory stops growing by about 3,000 directories a day. A test killed before its
  guard drops leaves its directory under `target/test-scratch/`, where a `cargo clean` removes it. Contributors get one helper
  to call, and one guard that names the fix. Editor users get hover in three more places and complete
  reference lists.
