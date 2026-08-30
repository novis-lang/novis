# Handoff

## State

**Stage 2 is closed.** ADR 0103 § 9's offline pair is on disk — `nvs config check [<file>...]` and
`nvs config dump [--origin] [--toml] [<file>...]` — and § 1 step 1's repeatable `--config` is a
global flag threaded into `nvs run`'s snapshot as well as into both audits. The driver's `command`
check (`docs/agent/loop-goal.toml:1731`) passes: `nvs config check tests/config/duplicate-key.toml`
exits 1 naming `duplicate-key.toml:4:1`.

`crates/nvs-cli/src/config.rs` is the new home of every CLI-side config concern — `LocalFiles`,
`boot_snapshot`, `check`, `dump` — moved out of `main.rs`, which keeps only the clap surface. Two
decisions live in its doc comments rather than in an ADR: the audit does **not** apply § 6's
ownership check (a machine auditing a tree is not the account that will serve it, so the check run
there refuses trees a server would accept and passes trees it will refuse), and `--origin` names the
**file** a key was written in and not the line (`Origin` carries a path and a `SourceId`; `toml::Value`
has no span once the document is parsed, and ADR 0103 § 9's example shows `prod.toml:4`).

**The driver's failing acceptance check is not a regression.** `native examples/limits.nvs
[4 capabilities]: exited 0, wanted non-zero` is item 11 unwritten: no memory cap is enforced and
`[capabilities]` is enforced nowhere, so the fixture holds its 512 slabs and exits 0 by design. It
closes in the group below, not before it.

`orient.py` printed neither ADR 0103 § 9 nor § 1, which are the sections this session's item named.
`[context] adrs` is documented as "the sections EVERY session reads", so a *per-item* section has no
home in the manifest and the item text naming one does not make the pack slice it — that is the gap,
and it costs a session one `peek.py` per section. `[context] modules` now names
`crates/nvs-cli/src/config.rs`, and the comment above it now covers the `budget.rs` warning too
(item 12's budget has no module yet; the selector is where it will be, exactly as `nvs-config`'s was).

## Next group

**Stage 4's capability gate — items 10 and 13, and this goal's one ADR slot.** File set:
`docs/adr/0118-*.md` (new), `crates/nvs-stdlib/src/registry.rs:708` (`CoreMethod`),
`crates/nvs-config/src/tree.rs:210` (`Capabilities`), `crates/nvs-config/src/snapshot.rs:53`
(`Snapshot::config`) and `crates/nvs-runtime/src/ctx.rs:867` (`set_config`, which is how a request
already holds the snapshot a gate must read).

- [ ] **ADR 0118 — where a capability check sits.** The goal's § *Standing decisions* reserves it and
      makes it the stage's first slice: what a capability is at the point of a call, where the check
      sits so no member can route around it, what it costs on a hot path, and how Stage 6's closure
      test knows a member needs one. Re-check the next free number before creating the file.
- [ ] **The gate itself**, per that ADR: a `CoreMethod` declares the capability it needs
      (`registry.rs:708`), the check reads `Snapshot::config`'s `Capabilities` (`tree.rs:210`) off the
      request, and an ungranted one throws naming it —
      `an_ungranted_capability_throws_naming_the_capability`, `loop-goal.toml:1790`.
- [ ] **`examples/capability.nvs`'s three frozen lines** (`loop-goal.toml:1807`): `granted: read ok`,
      `denied: fs.write`, `denied: script.spawn`. The path-bearing half is canonicalise-then-prefix,
      which item 6 already wrote once — do not write a second comparison.

## Backlog

- **ADR 0103 § 8, "the CLI flag list is closed at the global layer"** — the last unread clause of the
  goal's stage 2 item 5. `--config` is global; whether anything else may be is § 8's.
- **Item 11, safepoint-driven limits** — what `examples/limits.nvs` is waiting for; own file set
  (`crates/nvs-host`, `crates/nvs-codegen/src/emit.rs`), so its own group after the gate.
- **`dump --origin` prints no line number** — a span per leaf would have to be carried through the
  merge; `crates/nvs-cli/src/config.rs`'s `dump` doc owns the reason.
- **`nvs ctl config`** — ADR 0103 § 9's third command, waiting on goal 6's socket.
- **Item 18's `Core\Secret::reveal()`** is still not in the registry (`Qual::Reveal` is decided).
