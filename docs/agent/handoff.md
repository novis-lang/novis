# Handoff

## State

**m6's *Verify* row "N concurrent isolates cannot together exceed the tree's memory, CPU or output
budget" is enforced in all three thirds**, which closes the group the last two sessions were
working. `[limits] max_output` is now a ceiling `Ctx` answers for, on the memory ceiling's exact
shape: `nvs_runtime::budget::written_bytes` is a second thread-local counter, `Ctx::output_base`
re-bases it at `Ctx::new`, and `Ctx::output_used`/`output_limit`/`over_output_limit`/`output_breach`
mirror their memory siblings. The charge is in `Ctx::write_output` **below the capture and above the
sink** — the doc comment there owns why a captured byte is not a response byte and is bounded by
`[limits] memory` instead. `Limit::Output` is the fourth variant and the breach is a branch in
`nvs_safepoint` beside memory's; `run_helper` deliberately does not ask it.

**The deadline is now the tree's own word, not a copy.** `Ctx::deadline` is
`Arc<AtomicU64>` and `Ctx::child`/`Ctx::isolate` clone the handle, so one `expire_deadline` store
stops every isolate — including one spawned *before* the timer fired, which the copy could never
reach, because the timer holds the root and no registry of live children exists to walk. That was
the design call item 12's doc comment left open; ADR 0006's "one ceiling to divide" decided it. The
field doc states what it spends: one allocation per request *tree* and one pointer hop on a poll
already amortised over `bounded_loop`'s batch.

**ADR 0020 § 1 was amended in place** to list `max_output` among the limits that reach the tier-1
handler, with a paragraph saying why there are two reserved slices and not one per limit. No new
ADR; the fold is the whole change.

**The driver's failing check is stage 5, the artifact cache, and nothing of ADR 0042 is on disk** —
no `cache.rs` in `nvs-cli`, no header type, no reader. `nvs-config` holds only the *key*
(`crates/nvs-config/src/cache.rs:110`) and the `[cache] dir` directive. That is an open item, not a
regression, and it is the next group.

`orient.py`'s `[context] modules` still names `crates/nvs-host/src/budget.rs`, which never existed;
the accounting this session extended is `crates/nvs-runtime/src/budget.rs`. The pack warns every
session and the manifest has not been fixed.

## Next group

**ADR 0042's on-disk artifact cache, which is the driver's outstanding acceptance check.** File set:
a new `crates/nvs-cli/src/cache.rs` and a new `crates/nvs-cli/tests/cache.rs`, with
`crates/nvs-config/src/cache.rs:110` (the key `BLAKE3(source_content ‖ env_hash)`),
`crates/nvs-config/src/tree.rs:581` (`[cache] dir`) and `crates/nvs-config/src/trust.rs:110` (the
directory check) behind them. Take them in this order — each later slice reads what the earlier one
wrote.

- [ ] **The header and the write path** — ADR 0042 §§ 1, 2 and 4: one immutable content-addressed
      file per unit under `[cache] dir`, a header carrying `magic`, `format_version`, `env_hash` and
      a `BLAKE3` of the payload, written to a temporary name in the same directory and published by
      one atomic rename with **no lock file, ever**. The key is already built —
      `crates/nvs-config/src/cache.rs:110` — and the directory is
      `crates/nvs-config/src/tree.rs:581`. Closes
      `the_cache_is_a_fan_out_of_immutable_content_addressed_files` and
      `a_concurrent_write_resolves_by_rename_with_no_lock_file`.
- [ ] **The read path, verified whole before a single page is executable** — ADR 0042 § 3, which
      `orient.py` already prints in full. `mmap` `PROT_READ`, check magic/version/`env_hash` (a
      mismatch is a *miss*, never an error), `BLAKE3` the payload, and only then `mprotect` to
      `PROT_READ | PROT_EXEC`; a bad entry is deleted and invisible to the script. No panic, no
      `FATAL`, no `Throwable`. The `env_hash` the header carries is
      `crates/nvs-config/src/cache.rs:96`, and the digest type beside it is
      `crates/nvs-config/src/cache.rs:47`. Closes
      `an_artifact_is_verified_whole_before_any_page_is_executable` and
      `a_tampered_artifact_is_rejected`.
- [ ] **The cache directory's trust check, and eviction off the request path** — ADR 0042 §§ 5-6,
      which is the same ownership-and-mode question ADR 0103 § 6 asks of a config file: reuse
      `nvs_config::trust::check` (`crates/nvs-config/src/trust.rs:110`) rather than writing a second
      one. Closes `a_world_writable_cache_directory_is_refused` and
      `eviction_is_piggybacked_and_off_the_request_path`.

`[context]` gaps for `docs/agent/loop-goal.toml`: `adrs` needs ADR 0042 §§ 1, 2, 4, 5 and 6 (only
§ 3 is listed, and the next group needs all of them); `modules` still needs
`crates/nvs-host/src/budget.rs` corrected to `crates/nvs-runtime/src/budget.rs`.

## Backlog

- `[limits] wall_time` has no timer: nothing samples a clock and sets the deadline word except a
  test — `crates/nvs-runtime/src/ctx.rs`'s *The request's deadline* section owns the gap.
- `[limits] cpu_time` has the same gap on the host side — `Ctx::cpu_limit`'s field doc.
- Item 18's `Core\Secret::reveal()` is not in the registry — `docs/plan/m6.md`.
- `Live::admit`'s same-class check is asked of the answer, not the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- A warm-cache CLI start under 10 ms (`tools/bench.py --warm-start`) needs the cache above first.
- ADR 0048's portable single-file executable is untouched — `docs/plan/m6.md`'s *Verify*.
