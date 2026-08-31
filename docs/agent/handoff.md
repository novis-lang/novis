# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running. Stage 2 is closed.** `Core\IO` has ten
members — `read`, `readText`, `lines`, `write`, `exists`, `size`, `remove`, `removeDir`,
`temporaryDir` and `within` — and `examples/files.nvs` prints its seven frozen lines, over an
`[[app]]` block of its own in `nvs.toml` granting `fs` unscoped, because the temporary root it works
inside is the host's path and not one this repository can write down.

**`lines` answers `Core\IO\Lines`, which is how the registry spells `Iterable<string>`.** The class's
own doc comment owns the decision under it: the lines are **read and held**, not streamed off the
open handle — `lines` is `slurp` plus `Core\Str::lines`' rule exactly as `readText` is `slurp` plus a
decode. `examples/files.nvs`'s comment claiming the file is not held was the wrong half of that and
is corrected. `crate::str::line_pieces` is now `pub(crate)` and divides octets rather than validated
text, so the two members cannot come to disagree about what a line is.

Three fixtures still owe configuration their stage must write, unchanged: `examples/http.nvs` names
`http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs` needs an `[[app]]` block
naming `examples/logging/handler.nvs`; and every fixture that reaches the world still owes its
`process.exec` / `net.connect` grants.

`orient.py` still prints two dead `[context] modules` selectors — `crates/nvs-host/src/pool.rs` and
`crates/nvs-host/src/stream.rs` match no module. Nothing is blocked.

## Next group

**Stage 3's first half — `Core\Process`, argv-only, behind a `process.exec` door.** One file set:
`crates/nvs-runtime/src/capability.rs`, a new `crates/nvs-stdlib/src/process.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/lib.rs`, and `examples/process.nvs`
beside the `nvs.toml` block it owes.

- [ ] **The `process.exec` door**, written beside `open_read` at
      `crates/nvs-runtime/src/capability.rs:101` and refusing the same way. `nvs_config::Cap::ProcessExec`
      already exists and is already path-scoped (`crates/nvs-config/src/capability.rs:116`), so this
      slice is the door and its refusal only — no configuration change.
- [ ] **`Core\Process::run`, and the result it answers.** ADR 0044 is the member list; the spec's own
      row is `docs/spec/01-core-library.md:1102` and points there. New module on `address_of`'s chain
      at `crates/nvs-stdlib/src/lib.rs:310`, rows in `CLASSES` at
      `crates/nvs-stdlib/src/registry.rs:984`, grant in `CAPABILITIES` at
      `crates/nvs-stdlib/src/registry.rs:1121`. **The decision it owes:** `examples/process.nvs:35`
      writes `$result->exitCode` and `->stdout` as *properties*, and a `Core` class has only
      `instance` methods (`crates/nvs-stdlib/src/registry.rs:919`) — decide whether the fixture reads
      `->exitCode()` or the class gains a property surface, and say which in the class's doc comment.
      Two named tests come with it: `there_is_no_shell_string_form_of_run_or_spawn` and
      `a_windows_batch_or_powershell_target_is_refused`.
- [ ] **`examples/process.nvs` green**, with an `[[app]]` block in the repository's own `nvs.toml`
      granting `process.exec` for the `target/debug` binary the fixture runs —
      `examples/process.nvs:28` is where it picks the platform's spelling of it. Its assets are
      already on disk (`examples/process/hello.nvs:1`, and `say.bat` beside it), and the three frozen
      lines are in `docs/agent/loop-goal.toml` (stage 3).

## Backlog

- `Core\Cli`'s eight named tests, stage 3's other half — `docs/adr/0086-core-cli-terminal-is-a-sink.md`.
- `examples/logging.nvs` owes an `[[app]]` block naming its handler — `docs/agent/loop-goal.toml`, stage 7.
- `examples/http.nvs` owes the `http://127.0.0.1:8099` origin — same file, stage 5.
- The two dead `[context] modules` selectors — `docs/agent/loop-goal.toml`.
- A streaming `lines` over a held handle, if anything ever needs one — the rejected alternative is
  argued in `crates/nvs-stdlib/src/io.rs`'s `LINES`.
