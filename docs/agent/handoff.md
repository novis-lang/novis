# Handoff

## State

**Goal 7 — ADR 0131's temporary-directory sweep — is at stage 2, with its first slice landed.**
`capability::temp_dir` creates under the root Novis owns: `[io] temp_root` when a tree names one,
else a `novis` subdirectory of the platform temporary directory, created private on first use. `[io]`
is a new block in `nvs_config::tree` and `io.temp_root` a `System`/`Boot` row in
`nvs_config::directive`; the member's capability check is unchanged, still asked of the entry path
after the name is chosen. Both tests the stage-2 check names for `-p nvs-runtime` are green.

**Nothing of §§ 3-5 exists yet** — no tracked list, no sweep of any kind, no `nvs tmp clean`, no
`[debug] keep_temporary`. The design is settled and is not reopened:
[ADR 0131](../adr/0131-a-temporary-directory-dies-with-its-script-and-the-sweep-never-throws.md) is
the specification, and its § 2 is what the landed slice implements.

Two facts the next slice would otherwise spend calls re-deriving. `crates/nvs-host/Cargo.toml` names
`nvs-runtime`, `nvs-config` and `nvs-codegen` and **not** `nvs-stdlib`, so the sweep itself belongs in
`nvs-runtime` and the isolate calls it; and that crate is `unsafe_code = "deny"`, not `forbid`, so a
fixture there may open a narrow `#[expect(unsafe_code, reason = …)]` the way `nvs-stdlib`'s do.

## Next group

**Stage 2's remainder: the tracked list, the end-of-script sweep, and the card that describes it** —
one file set: `crates/nvs-runtime/src/ctx/`, `crates/nvs-runtime/src/capability.rs`,
`crates/nvs-host/src/isolate.rs`, `crates/nvs-stdlib/src/io.rs`.

- [ ] **The runtime records each path `temporaryDir` answers** (0131 § 3) — a per-script list on
      `Ctx`, request-local and O(directories created) per ADR 0004. The one producer is
      `crates/nvs-runtime/src/capability.rs:773`; the field joins the block at
      `crates/nvs-runtime/src/ctx/mod.rs:991`, and `crates/nvs-runtime/src/ctx/hooks.rs:229` is the
      neighbouring per-script queue to shape it after.
- [ ] **The sweep, after the last user code** (0131 § 3) — never throws, silent about a path already
      gone, one `crates/nvs-runtime/src/floor.rs:87`-style log record for a deletion the OS refused.
      It runs at `crates/nvs-host/src/isolate.rs:305`'s tail, which is after
      `crates/nvs-stdlib/src/script.rs:449` has drained the `onExit` queue on a CLI ending. The four
      tests the check names are `-p nvs-host`; `a_refused_deletion_logs_and_never_throws` holds the
      handle itself, per the goal's standing decision on Windows refusals.
- [ ] **The registry card, in the same slice** (ADR 0117) — `crates/nvs-stdlib/src/io.rs:999`'s
      `ret` still reads "Removing it is the program's own job", which the sweep makes false. Its
      `short` already names the owned root. `docs/novis.md` regenerates from it under `verify.py`.

## Backlog

- § 4's orphan sweep at `nvs serve` boot, keyed on owner liveness — stage 3 of
  [docs/agent/goals/7-temp-sweep.md](goals/7-temp-sweep.md).
- `nvs tmp clean`, with a dry run that deletes nothing — stage 4 of the same file.
- `[debug] keep_temporary`, reloadable, one log line per kept path — stage 5, ADR 0131 § 5.
- § 4 leans on the root being exclusively Novis's, and `temp_dir` adopts an existing root without
  examining it — the gap is stated in that function's own doc comment and is the sweep slice's to
  close.
- **`[context] adrs` gap:** the pack printed ADR 0131's *In short* alone, so §§ 2 and 3 were sliced
  by hand. That list in `docs/agent/loop-goal.toml` should name `## 2`, `## 3`, `## 4` and `## 5` —
  every remaining slice of this goal reads one of them.
