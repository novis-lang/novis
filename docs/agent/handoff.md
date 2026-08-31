# Handoff

## State

**ADR 0086 § 5's live region is on disk, and § 8's restoration is a `Drop`.** `Core\Cli::live<T>`
and `Core\Cli::progress<T>` are rows on `CLASS` (`crates/nvs-stdlib/src/cli.rs:163`) over one
`nvs_runtime::terminal::Region` (`crates/nvs-runtime/src/terminal.rs:765`), which hides and restores
the cursor, coalesces frames onto `FRAME_INTERVAL`, diffs against what is on screen, clamps a row to
the terminal's width, and **renders nothing at all** where `is_interactive()` is false. Restoration
is that type's destructor and lives nowhere else; the *scope* is `cli.rs`'s `Open` guard over a
per-core stack of open regions, so a throw, a fatal and an internal panic all close a region the
same way. The driver's § 5 check passes.

**A handle names a depth, not a region.** `Core\Cli\Live` and `Core\Cli\Progress` each carry the
stack depth their `live`/`progress` call pushed, and each refuses with a `LogicError` unless that
depth is still the innermost one — which covers both the escaped handle and the outer handle painted
while an inner region is open, since those are one rule. Six `.nvst` cases cover the two members and
the two handles.

**Not done in § 5**: repainting on a resize. ADR 0086 § 3 freezes the profile for the process, so a
region's width is fixed for its life, and the signal that would say otherwise is `Core\Signal`'s,
which is not built — `terminal.rs`'s `Region` doc owns that gap.

**Still owed on `Core\Cli`**: `arguments`, `write`, `displayWidth`, `multiSelect` and § 4's scripted
answer queue for `nvs test` — `cli.rs`'s gap 2 owns them. `Text + Text` still needs a row in
`nvs_types`' operator table (`crates/nvs-types/src/expr/operators.rs`). Stage 2 owes `truncate`,
`lock` and `Core\IO::stdin`/`stdout`/`stderr`.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field; § 15's `Core\Cli` list is what these signatures are written against.

## Next group

**§ 4's fifth prompt and its test-mode answer queue, over one file set:
`crates/nvs-stdlib/src/cli.rs` and `crates/nvs-runtime/src/terminal.rs`.**

- [ ] **`multiSelect<T>` beside the four prompts** — ADR 0086 § 4. A row on `CLASS`
      (`crates/nvs-stdlib/src/cli.rs:163`) with its `address` arm
      (`crates/nvs-stdlib/src/cli.rs:840`) and its body in the prompts section
      (`crates/nvs-stdlib/src/cli.rs:995`); it is `select<T>`
      (`crates/nvs-stdlib/src/cli.rs:1255`) with a second reading loop, because its answer is a set
      rather than a value, so the labels come from the same `label_of` and only the parse of the
      answer differs.
- [ ] **A scripted answer queue under `nvs test`** — ADR 0086 § 4's last paragraph. Today a prompt
      in a test takes the not-interactive path, because a test's output is a buffer and `watched`
      (`crates/nvs-stdlib/src/cli.rs:1030`) answers `false`; the queue belongs beside `ask_terminal`
      (`crates/nvs-stdlib/src/cli.rs:1037`), which is the one place a prompt decides where its
      answer comes from.
- [ ] **`write` and `displayWidth`** — ADR 0086 § 3, `cli.rs`'s gap 1. `write` is a row and a
      `Cli\Stream` argument over the sink `echo` already is; `displayWidth` owes a UAX #11 table and
      is what `terminal.rs`'s `clamp` (`crates/nvs-runtime/src/terminal.rs:928`) approximates with a
      character count today.

## Backlog

- `Text + Text` needs a row in `nvs_types`' operator table — `crates/nvs-types/src/expr/operators.rs`.
- Stage 2 owes `truncate`, `lock` and `Core\IO::stdin`/`stdout`/`stderr` — `crates/nvs-stdlib/src/io.rs`.
- `Core\Cli::arguments` — spec § 15, `cli.rs`'s gap 2.
- A region does not repaint on a resize — `terminal.rs`'s `Region` doc.
- `docs/spec/01-core-library.md` is in no `[context]` field of `docs/agent/loop-goal.toml`.
