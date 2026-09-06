# Handoff

## State

**Goal 7 — ADR 0131's temporary-directory sweep — has §§ 2, 3 and 5 on disk and green, plus § 4's
predicate.** `capability::temp_dir` creates under the owned root and records what it hands back;
`sweep::at_script_end` deletes the recorded list from `Ctx::drop` and nowhere else; `[debug]
keep_temporary` branches that into one `info` line per kept path and no deletion at all; and
`sweep::owner_is_alive` answers § 4's liveness question over one path, `kill(pid, 0)` on Unix and
`OpenProcess` on Windows. That module's own doc is the home of every rule above, including why every
ambiguity answers *alive*.

**Nothing of § 4's two walks exists** — no `nvs serve` boot sweep and no `nvs tmp clean`. The
predicate they will share is written and has no caller.

**Stage 2's and stage 5's checks are filed `-p nvs-runtime` now**, each with a comment saying why
nvs-host cannot host them, in both `docs/agent/loop-goal.toml` and `docs/agent/goals/7-temp-sweep.toml`.
Stage 3's and stage 4's checks still name `-p nvs-server` and `-p nvs-cli` and have not been triaged.

## Next group

**§ 4's orphan sweep: the walk, then its two doors** — one file set:
`crates/nvs-runtime/src/sweep.rs`, `crates/nvs-runtime/src/capability.rs`,
`crates/nvs-cli/src/main.rs`, `crates/nvs-cli/src/serve.rs`, `docs/agent/loop-goal.toml`.

- [ ] **The walk over the owned root, beside the predicate it reads** (0131 § 4) — one function that
      lists the root and answers the entries whose owner is dead, so both doors below share it rather
      than agreeing today. It goes beside `crates/nvs-runtime/src/sweep.rs:215`, and it needs the
      root: `temp_root` at `crates/nvs-runtime/src/capability.rs:840` is private and taking a `&Ctx`,
      so the second edit of this slice is deciding whether the walk takes a `&Path` (it should — § 4's
      callers each know their own root, and the predicate beside it already takes nothing else).
- [ ] **`nvs tmp clean`, the operator's door** (0131 § 4) — the subcommand dispatch is
      `crates/nvs-cli/src/main.rs:912`'s `runtime_commands`. It prints each path it removes, supports
      `--dry-run`, and has no force flag: the worst it may do is nothing. The three names are at
      `docs/agent/loop-goal.toml:4362` and are filed `-p nvs-cli`, which is where the command lives.
- [ ] **The `nvs serve` boot sweep** (0131 § 4) — once before traffic and on no other invocation.
      `crates/nvs-cli/src/serve.rs:672` is where the boot assembles its mounts. The check at
      `docs/agent/loop-goal.toml:4347` names `-p nvs-server` for it; triage that filing before writing
      the test, the way the two checks this session moved were triaged — the boot is `nvs serve`'s and
      `crates/nvs-server` may not be able to see the configured root at all.

## Backlog

- Stage 3's two request-side names — a request's dirs swept after its `afterResponse` work, and an
  aborted request's swept by the surviving worker (`docs/agent/loop-goal.toml:4347`).
- Stage 5's remaining checks: the fixture's directory is gone after the run, and the two suites
  (`docs/agent/loop-goal.toml:4374`).
- `[context] adrs` now names `0131 §3`/`§4`/`§5` beside the bare `0131`, which printed only the ADR's
  *In short*; if the next pack still prints one section, the selector syntax is the thing to fix
  (`docs/agent/loop-goal.toml:90`).
