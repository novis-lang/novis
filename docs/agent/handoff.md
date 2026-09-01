# Handoff

## State

**§ 14 is closed, and its last line is one member rather than three.** `Core\IO::stdin` answers a
`tainted string` — everything the invoker attached, read to end of input in one call.
`Core\IO::stdout` and `::stderr` deliberately do not exist.

**The decision has three homes and is restated in none of them twice.**
`nvs_core_io_stdin`'s doc comment carries all of it: why there is no writing half (ADR 0086 § 1's
substitution is uniform, so a raw descriptor onto standard output is the way *around* the sink, and
`Core\Cli::write` already is that operation under ADR 0063), why the reading half is a value and not
a `Core\IO\File` (six of that class's nine members are meaningless or destructive on an inherited
descriptor, and its `close` would take the process's stream away from every later request sharing
it), and why a terminal is waited on rather than refused. ADR 0086 § 7 gained the one bullet, and
spec § 14's standard-stream line was rewritten to match.

**The one thing this member does not answer is a served request.** A handler calling it parks its
core on a descriptor the request never opened. A refusal was drafted and taken back out: no case can
present a terminal, so the guard was a message the error-path gate is right to call owed. Goal 6 is
where a case can serve a request and therefore where the rule can be asserted rather than guessed —
`nvs_core_io_stdin`'s doc comment carries that in full and is the only home for it.

**The member needs no capability and says so as a `None` row** in `registry::CAPABILITIES`: standard
input is a descriptor the process was started holding, so ADR 0118 § 1 has no door to put a check
at — the handle members' reading, one step earlier, since here there was never even a path.

**The tainted roster is closed and now eight.** `nvs-types`' `core_lib.rs` holds it as an equality
over every `Core` member whose *answer* is qualified, with the reasoning per member in its own doc
comment — a new tainted answer fails that test rather than passing unnoticed, which is what it is
for.

**Conformance can ask this member anything.** `nvs-test` spawns through `Command::output`, which
gives a case `Stdio::null` for standard input, so `stdin()` returns the empty string at once and no
case can hang the suite.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II. It cannot pass inside this goal: §§ 15-19 are `Core\Request` and its
neighbours, which are goal 6's.

**Two pack gaps.** `[context] modules` does not select `nvs-runtime/src/terminal.rs`, whose
`profile()` and `Stream` this member calls and whose UAX #11 gap is the next group's first slice.
And `[context] playbook` filters to the *item's* two anchor paths, so a group that also writes
`.nvst` cases never sees the case-authoring bullets — `===` is `E0232` is already one of them, and it
cost a run here.

## Next group

**`Core\Cli::displayWidth` — ADR 0086 § 3's last table row and `cli.rs`'s own known gap 1 — over
`crates/nvs-runtime/src/terminal.rs`, `crates/nvs-stdlib/src/cli.rs` and
`tests/conformance/core/`.**

- [ ] **The UAX #11 table `cli.rs` says this tree does not carry** — east-asian wide and fullwidth,
      zero-width combining marks, and what a control byte counts as after § 1's substitution has
      replaced it with a picture. `crates/nvs-runtime/src/terminal.rs:920` is where the note lives
      and beside where the answer belongs.
- [ ] **The member itself** — `displayWidth(string $value): uint`, ADR 0086 § 3. Row beside
      `colorDepth` at `crates/nvs-stdlib/src/cli.rs:231`, card in row order, helper beside
      `crates/nvs-stdlib/src/cli.rs:1306`, `address` arm at `crates/nvs-stdlib/src/cli.rs:1020`. **No
      `CAPABILITIES` row**: `Core\Cli` has none at all, so the class is not capability-bearing and
      the table's per-member rule does not reach it. Clear known gap 1 at
      `crates/nvs-stdlib/src/cli.rs:125` and the roster line at `crates/nvs-stdlib/src/cli.rs:159`.
- [ ] **Three `cli-` cases** under `tests/conformance/core/`, each asking a different question: a
      width that is not a length (a wide glyph), a bound asserted on both sides (a combining mark
      adds nothing), and agreement with what `Core\Str::length` answers where the two must differ.
      `crates/nvs-stdlib/src/str.rs:1` is the member the third one compares against, and
      `tests/conformance/core/cli-a-process-with-no-terminal-measures-eighty-by-twenty-four.nvst:1`
      is the nearest case in shape.

## Backlog

- Reading `[log] target` — stage 7, `docs/implementation-plan.md`'s *Open now*.
- `Core\Mail`'s TLS, `AUTH` and `list` — stage 9, same field.
- Stage 10's `every_part_two_spec_member_is_registered` — blocked on §§ 15-19, goal 6's.
- `cli.rs`'s known gaps 2 and 3 — that module doc owns both, and neither closes from inside it.
- `Core\IO::stdin` inside a served request — goal 6's, per `nvs_core_io_stdin`'s doc comment.
