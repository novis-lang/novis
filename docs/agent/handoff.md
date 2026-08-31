# Handoff

## State

**The failing acceptance check is closed.** `nvs-host (the tier-3 handler script)` named a test that did
not exist; it is now in `crates/nvs-host/tests/limits.rs`, and it asks the **ladder** rather than the
constructor its sibling asks — `nvs_host::ladder::escalate` over a request already past `[limits] memory`,
through ADR 0118 § 2's spawn door, to a program supplied by a `nvs_runtime::script::Resolver` the case
installs with `script::scoped`. The readings that matter are taken *inside* the handler: it spends a
megabyte of the reserve and polls, because a reserve is a ceiling and a handler that only reads one cannot
tell a widened ceiling from one it never needed. `Output::Inherit` puts its bytes on the failing program's
own stream, which the case reads back through `Ctx::take_buffered_output`.

**`Core\Cli::write` has landed** — ADR 0086 § 3's first row, `write(string|Cli\Text $value, {stream?:
Cli\Stream, newline?: bool}): void`. It performs § 1's substitution table rather than carrying a second
copy, and recognises the carrier by its class exactly as `nvs_runtime`'s `nvs_echo_value` does. Its subject
is the file's one `CoreTy::Union` of a `Text` and an `Instance`; `Qual::Neutral` is § 1's "regardless of
qualifier" in the type system, since the neutralizing *is* the laundering. `Stream::Out` is
`Ctx::write_output` (so a `Core\Out::capture` takes it) and `Stream::Err` is `Ctx::write_diagnostic` (so a
capture does not); `Stream::In` throws `LogicError`, because `Cli\Stream` is one enum so that `isTty` can
ask about all three.

**Still owed on `Core\Cli`**: `arguments` and `displayWidth` — the module's own gaps 1 and 2. Stage 7 still
owes reading `[log] target`; nothing else changed there.

`orient.py` printed no section of ADR 0086 although the whole item was § 1's sink half: `[context] adrs`
needs `0086:1` and `0086:3`, and `[context] spec` (or its equivalent) still misses
`docs/spec/01-core-library.md` § 15, which is the one place `write`'s place in the roster is written down.

## Next group

**`Core\Cli`'s two remaining members, over one file set: `crates/nvs-stdlib/src/cli.rs` and
`tests/conformance/core/`.**

- [ ] **`Core\Cli::arguments`** — spec § 15's `arguments(): array<tainted string>`, the raw `argv` a
      program reads when it declares no `#[Command]`. The five edits of a `Core` member, at
      `crates/nvs-stdlib/src/cli.rs:169` (the rows, and it goes *first* — spec order) and
      `crates/nvs-stdlib/src/cli.rs:992` (the `address()` arm). It reads no OS: ADR 0118 § 2 keeps `argv`
      out of this crate, so the seam is `crates/nvs-runtime/src/ctx.rs:1302`'s `Ctx::set_command_line`,
      which `nvs-cli` already fills beside `set_program_name` (`crates/nvs-cli/src/main.rs:851`) — check
      whether a reader exists there before adding one. Every element is `tainted`.
- [ ] **`Core\Cli::displayWidth`** — ADR 0086 § 3's `displayWidth(string $value): uint`, UAX #11 columns,
      sited here and not on `Core\Str` because a width is the renderer's and not the string's. The same
      two anchors as above — `crates/nvs-stdlib/src/cli.rs:169` for the rows and
      `crates/nvs-stdlib/src/cli.rs:992` for the `address()` arm.
      It owes a UAX #11 table this tree does not carry: a `unicode-width` dependency is
      pre-authorized under ADR 0051 § 4 and owes the `[workspace.dependencies]` comment, `cargo deny
      check` and `python tools/gen-attribution.py`.

## Backlog

- Stage 7 § 4: something has to read `[log] target` — `docs/adr/0106` § 10, `crates/nvs-runtime/src/logfile.rs`.
- `Text + Text` needs a row in `nvs_types`' operator table — `crates/nvs-stdlib/src/cli.rs`'s module doc.
- `Core\IO`'s `truncate` and `lock` — `crates/nvs-stdlib/src/io.rs`.
- ADR 0092 § 2's `Cli\Text` of runs, which is what a per-stream render would need — `cli.rs` gap 3.
