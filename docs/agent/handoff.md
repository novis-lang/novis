# Handoff

## State

**Goal `unowned-closures`, stage 5 is closed: both of its checks are green.** `env_hash`
(`crates/nvs-config/src/cache.rs:147`) now folds the running compiler binary rather than the package
version — `build_stamp` hashes this executable's path, byte length and modification time, once per
process behind a `OnceLock` — so two builds of one unreleased version key their units apart. The one
case the stamp cannot separate is two builds that could neither name nor examine themselves; that is
closed a layer up by `build_is_identified`, which `nvs_cli::cache::from_config`
(`crates/nvs-cli/src/cache.rs:1339`) asks before it opens any directory, so a *configured*
`opcache.file_cache_dir` is now covered exactly as the default one is. `default_dir` therefore no
longer files artifacts under a per-build directory: one directory, one § 6 eviction budget, and no
dead build's artifacts left where nothing reclaims them. `rule:config/the-extension-set-is-in-every-unit-key`
states what `compiler_version_hash` is; `rule:config/opcache-file-cache-directives-are-system` no
longer calls the default location per-build. Nothing is blocked.

**What remains of this goal is stage 6, the register**: `python tools/owners.py` reports
`unowned: 44` and the check wants `unowned: 0`. Each is built, or tagged with the milestone that
cannot be reached without — the standing decision's test for an honest deferral.

## Next group

**Stage 6: the two definite-assignment scanners** — one file set: `crates/nvs-types/src/ctor_init.rs`
and `crates/nvs-types/src/lateinit.rs`, whose gap 2 is one shared limitation written twice.

- [ ] **`scan_expr` descends into every composite expression a constructor can hide an assignment
      in** — `crates/nvs-types/src/ctor_init.rs:51`,
      `rule:classes/definite-property-initialization`. It descends into a handful of forms today, so
      an assignment inside an unlisted composite is invisible and the pass reports a property as
      unassigned that a path does assign. The rule's own mandate is no false positives, so widening
      the walk is the direction; the list of forms the AST can hold is `nvs_syntax`'s, and the walk
      must stay total over it rather than enumerating favourites.
- [ ] **The same walk, once, for the `lateinit` read-before-write check** —
      `crates/nvs-types/src/lateinit.rs:48`, `rule:classes/lateinit-read-before-write`. Its gap 2 is
      the same sentence: the two scanners were written apart and drifted into one duplicate
      limitation. Whether they become one shared walk or two callers of one descent is this group's
      own call — the module docs say they already share the definite-assignment idea.

## Backlog

- 42 further unowned module-doc gaps — `python tools/owners.py --unowned`, reasons in
  `docs/agent/carried-gaps.md` § *Unowned*.
- `crates/nvs-ir/src/lib.rs` holds ten of them (gaps 2, 3, 4, 5, 6, 9, 15, 16, 17, 19, 20) — the
  largest single file set left in stage 6.
- `crates/nvs-lsp/src/index.rs` gaps 1–4 are one file set: occurrences the index does not record.
- `crates/nvs-cli/src/openapi.rs` gaps 1–5 are one file set — `docs/agent/carried-gaps.md` says the
  `nvs/rest` package is unscheduled, so these may be a deferral rather than a build.
- One gap is owned by a retired goal (`python tools/owners.py`, `retired-owner: 1`) — a session
  decides it.
