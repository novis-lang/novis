# Handoff

## State

**ADR 0106 § 10 is closed end to end — both bounds sit on the sink, not on either caller.**
The rate limit is `nvs_runtime::floor::report`
(`crates/nvs-runtime/src/floor.rs:161`): a per-core window keyed by a hash of the record's own
JSON rendering, so identical records inside `COALESCING_WINDOW` (1s, no directive) become one
line and the next occurrence after the window carries `nvs_render::Envelope::count`. The window
holds **a key, a deadline and a count — never record content**, because it outlives the request
that opened it; the cost is that a burst which stops is under-reported by its tail, which
`admit`'s doc comment owns. `count` is an envelope key rather than a `fields` entry because the
sink writes it and a producer's bag is the call site's vocabulary — ADR 0092 § 1's table now
carries the row, and that ADR gained its first `Amended by`.

Rotation is `crates/nvs-runtime/src/logfile.rs`, reached as `OutputSink::File`: the bound is the
product `(keep + 1) * max_bytes` and nothing else, the bounds are constructor arguments so a
case need not write 8 MiB, and the file is closed before the renames because Windows refuses to
rename an open one. **Nothing reads `[log] target` yet**, so the variant is reachable only
through `Ctx::set_diagnostic_sink` — that plumbing is the remaining half of stage 7 § 4.

`the_engine_floor_rotates_and_rate_limits_itself` (`crates/nvs-stdlib/src/log.rs`) is the
acceptance check the driver reported missing, and it asserts both bounds in one function
because the check names one test. **Still owed on `Core\Cli`**: `arguments`, `write` and
`displayWidth` — `cli.rs`'s gaps 1 and 2, untouched again.

The orientation pack printed no section of ADR 0106 although the whole group is § 10:
`[context] adrs` needs `0106:10`. It still prints no section of ADR 0020 and no
`docs/spec/01-core-library.md`.

## Next group

**`Core\Cli`'s three remaining members, over one file set: `crates/nvs-stdlib/src/cli.rs`,
`crates/nvs-stdlib/src/registry.rs` and `tests/conformance/core/`.**

- [ ] **`Core\Cli::write`** — ADR 0086 § 1's sink half: the member that writes a `Cli\Text`
      to the terminal with the substitution the sink already owns.
      `crates/nvs-stdlib/src/cli.rs:1` is the class, and the five edits of a `Core` member
      apply unchanged.
- [ ] **`Core\Cli::arguments`** — the raw `argv` a program reads when it declares no
      `#[Command]`, beside `crates/nvs-stdlib/src/command.rs:1`'s compiled table.
- [ ] **`Core\Cli::displayWidth`** — ADR 0086 § 2's width in terminal cells, over
      `crates/nvs-stdlib/src/granularity.rs:1`'s unit question rather than a second one.

## Backlog

- `[log] target` and `[log] format` read at run time — `crates/nvs-runtime/src/floor.rs`'s
  module doc names both as unread.
- `Core\IO::truncate` and `::lock` — the plan's *Open now*, stage 2's handle half.
- ADR 0106 § 10's tail count, if a later session wants it — `admit`'s doc comment says what it
  would cost.
