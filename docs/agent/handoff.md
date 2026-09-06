# Handoff

## State

**Goal 7 — ADR 0131's temporary-directory sweep — has §§ 2-5 on disk, and every test its acceptance
list names is now in the tree**: 610 `cargo-named` tests and 183 `.nvst` cases, none missing. The last
one, `a_finished_scripts_temporary_dir_is_gone_from_the_owned_root`, landed this session at
`crates/nvs-cli/src/script.rs:1079` — a whole script compiled from a file, run as an isolate under a
configured `[io] temp_root`, with the directory it filled gone afterwards and the owned root left
standing and empty.

That end-to-end reading rests on two facts the case does not arrange and deliberately does not mock:
`Ctx::isolate` copies the parent's configuration (`crates/nvs-runtime/src/ctx/isolate.rs:356`), so the
child reads the same `[io] temp_root`, and `Ctx::drop` is the sweep's only caller
(`crates/nvs-runtime/src/ctx/mod.rs:1172`), so the context `nvs_host::Isolate::run` makes and drops is
what takes the directory away. Nothing in `nvs-cli` calls the sweep; the isolate ending is the call.

**Whether the goal is met is the driver's gate to say.** Every named test and case exists, but the
list also carries 17 `command`, 53 `exact` and 29 `nvs-suite` checks that only a real run decides, so
the status line is `CONTINUE` and `tools/loop.py`'s own acceptance pass is what ends the run.

## Next group

**§ 5's mirror of § 3, in the file that just grew § 3's.** One file:
`crates/nvs-cli/src/script.rs`.

- [ ] **`[debug] keep_temporary = true` end to end** (0131 § 5) — the same fixture and the same helper
      as the case below it, and the temporary directory *still there* after the script ends: the
      negative reading that makes the positive one mean something, since a sweep that never ran at all
      passes § 3's case only by accident of the root being empty for another reason.
      `crates/nvs-cli/src/script.rs:1042` is `rooted_at`, which grows a second argument for the key,
      and `crates/nvs-cli/src/script.rs:1079` is the case to mirror. The branch under test is
      `crates/nvs-runtime/src/sweep.rs:90`; asserting the *kept path is named in the log* is the
      harder half — it goes through `crate::floor::report`, so start with the directory surviving and
      add the record only if this crate can already read one.
- [ ] **The plan's `Open now` says nothing about ADR 0131** — `docs/implementation-plan.md:44`. The
      field is 1998 B of its 2000 B ceiling, so this is a `## plan-edit:` that **replaces** a sentence
      with one about the sweep, never one that adds a sentence; an addition alone is refused.

## Backlog

- `[debug] keep_temporary` has no CLI-layer reading at all — `crates/nvs-cli/src/script.rs` owns it.
- The goal's completion is decided by `python tools/loop.py --goal-only` and by nothing a session
  writes — `docs/agent/loop-goal.toml` is the list.
- `benches/abi-probe/tests/` is part of this goal's acceptance list — see the playbook bullet.
