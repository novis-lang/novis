# Handoff

## State

**Goal `unowned-closures`. The register is `unowned: 15`** (`python tools/owners.py`), `--deferrals`
green, 45 items still owned by this goal. The 15 unowned are the scheduling questions and none of them
is this goal's own gap.

**`nvs-db` owes nothing.** Both of its `# Known gaps` blocks are gone: `schema.rs` refuses a key over a
column of a backend's unbounded type, and `ddl.rs` emits a SQL Server default change. What each
replaced them with is the module doc's own prose — `crates/nvs-db/src/schema.rs:51` and
`crates/nvs-db/src/ddl.rs:85` are where those sections now sit.

**Stage 5's remaining items are outside `nvs-db`**: `crates/nvs-cli/src/cache.rs` gaps 1–2, and
`crates/nvs-server/src/{route,schedule,trace}.rs` gap 1 with `crates/nvs-runtime/src/metrics.rs:105`
gap 1 beside them. The stage's `nvs-config` and `nvs-server/src/bounds.rs` entries are already closed —
the goal prose's list is ahead of the register there, and the register is what is true.

## Next group

**Stage 5: the artifact loader's two platform gaps** — one file set: `crates/nvs-cli/src/cache.rs`,
`crates/nvs-codegen/src/lib.rs`. Both items share one `Decided:` sentence, so the second is nearly free
once the first lands. `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`
is what the loader implements and
`rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` what makes a page runnable.

- [ ] **One shared "make executable" in `nvs-codegen`, and `aarch64` loads through it** —
      `crates/nvs-cli/src/cache.rs:166`'s gap 1 is the `Decided:` sentence: freshly written bytes need
      instruction-cache maintenance `mprotect` does not imply, and the loader has no home for it.
      `crates/nvs-cli/src/cache.rs:431` is where the loader makes the mapping executable and
      `crates/nvs-cli/src/cache.rs:339` (`relocate`) is what runs before it;
      `crates/nvs-codegen/src/lib.rs:971` is the JIT's own note on the same step, which is the function's
      home. `HOST_ARCH` then answers for `aarch64` and the payload stops being a miss on every run.
- [ ] **Strip Mach-O's leading underscore where a symbol is resolved** —
      `crates/nvs-cli/src/cache.rs:174`'s gap 2, latent today because CI's Mach-O host is `aarch64`:
      an undefined name arrives as `_nvs_echo_str` and `resolve` answers `None`, so every artifact is a
      miss on an x86-64 Mac. It is the same `Decided:` sentence as the item above and belongs in the
      same slice's file set.

## Backlog

- `crates/nvs-server/src/route.rs:32` gap 1 — where a forged CSRF token is refused (stage 5).
- `crates/nvs-server/src/schedule.rs:80` gap 1 — a fire's context carries the deployment's configuration.
- `crates/nvs-server/src/trace.rs:78` gap 1 — the events a sampled request files without `DebugFlags::TRACE`.
- `crates/nvs-runtime/src/metrics.rs:105` gap 1 — the `otlp` pusher behind `[metrics] endpoint`.
- Spec § 13's `Core\Test` cell spells `request`'s bag nowhere; `crates/nvs-stdlib/src/test.rs`'s
  `REQUEST_OPTIONS` is the roster it should state (`docs/agent/carried-gaps.md`).
- Stage 6, the deferrals, is untouched and `python tools/owners.py --deferrals` is already green.
