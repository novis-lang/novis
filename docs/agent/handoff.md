# Handoff

## State

**§ 14's handle roster is complete.** `Core\IO\File` carries all nine of its members: `truncate`
and `lock` landed this session with three conformance cases each, under
`tests/conformance/core/io-*.nvst`. What is left of § 14 is `Core\IO::stdin`/`stdout`/`stderr`.

**`lock` is exclusive, never waits, and is released by `close`.** Contention is an `IOError`
rather than a `false`, and there is no `unlock` because the lock belongs to the descriptor.
`nvs_core_io_file_lock`'s own doc comment is the home of all four decisions and none is restated
here. What it deliberately does not promise is whether a lock stops a *non-holder's* reads and
writes — advisory on Unix, mandatory on Windows — so no case freezes that.

**Neither member declares a capability**, and each says so as a `None` row in
`registry::CAPABILITIES`: the descriptor is the grant `Core\IO::open` already checked, and
re-checking the path would check a name that may since have been renamed away from the file.

**The driver's acceptance check still names `every_part_two_spec_member_is_registered`**, which is
not written: it is stage 10's gate over a *complete* Part II, not a regression.

**Two pack gaps, both in the previous handoff rather than in `[context]`.** The item said
`truncate` "needs no new `CAPABILITIES` row" — the gate requires a `None` one. And it named
`tests/conformance/io/`, which does not exist: `Core\IO`'s cases are `tests/conformance/core/io-*`.

## Next group

**§ 14's three standard streams — stage 2's remainder — over `crates/nvs-stdlib/src/io.rs`,
`crates/nvs-stdlib/src/registry.rs` and `tests/conformance/core/`.**

- [ ] **`Core\IO::stdout` and `::stderr`** — spec § 14's last line, as two rows on the `Core\IO`
      static roster at `crates/nvs-stdlib/src/io.rs:81`, answering the `FILE` class at
      `crates/nvs-stdlib/src/io.rs:761`. **Decide the sink question before writing the row**: ADR
      0086 makes the terminal a sink and `Core\Cli::write` at `crates/nvs-stdlib/src/cli.rs:1159`
      performs § 1's substitution table, so a `File` over fd 1 whose `write` skips it is both a
      second way to one operation (ADR 0063) and a hole in that sink.
- [ ] **`Core\IO::stdin`** — the reading half, same two anchors at
      `crates/nvs-stdlib/src/io.rs:81` and `crates/nvs-stdlib/src/io.rs:761`. `Core\Cli`'s § 4
      prompts already read the terminal under a deadline, so this slice says which of the two owns
      an interactive read rather than adding a second one.
- [ ] **Three `.nvst` cases per member**, under `tests/conformance/core/` with an `io-` prefix.
      The floor at `crates/nvs-stdlib/tests/conformance_coverage.rs:286` is three *questions* per
      member and `cargo test -p nvs-stdlib` fails below it;
      `crates/nvs-stdlib/src/io.rs:1691` is the shape a new member copies, cards and all.

## Backlog

- `Core\Cli::displayWidth` is owed — docs/implementation-plan.md, *Open now*, stage 3.
- `Core\Storage`'s TLS, `AUTH` and `list` are owed — the same field, stage 9.
- Stage 10's six gate tests are unwritten and are what closes this goal —
  docs/agent/loop-goal.toml:2565.
- A *shared* lock, if one is ever wanted, arrives as an R11 enum and not as a `bool` —
  `nvs_core_io_file_lock`'s doc comment says why.
