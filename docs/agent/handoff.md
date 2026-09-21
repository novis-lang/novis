# Handoff

## State

Goal `core-cli-progress-and-6-more`: five of its sixteen features are done —
`Core\Cli\Progress::advance`, `Core\Cli\Style::of` and all three of `Core\Cli\Text` each carry an
`about.md`, three examples with blessed `.out` files, one attack, one bench with a judged
`allocations` declaration, and a Rust `#[test]` with its `covers:` marker. `python
tools/dossier.py --group 'Core\Cli\Text'` shows every column filled, and `--id '<feature>'` says
`complete.` for all five.

Nothing is blocked. The remaining eleven items are `Core\Command`'s three members in `command.rs`,
then the eight of `Core\Compress` and its two stream classes in `compress.rs`.

## Next group

**`Core\Command`'s three members** — one file set, none of it opened yet:
`crates/nvs-stdlib/src/command.rs`, `docs/examples/core/Command/`, `tests/hostile/core/Command/`,
`benches/members/core/Command/`. All three are `rule:testing/feature-proofs`, and the five landed
`Core\Cli` features are the model for what each proof looks like. `Core\Command` is the compiled
command table, so the three share one fixture: a program that declares commands and then asks the
table about them.

- [ ] **`Core\Command::help`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/command.rs:115`. It renders the table a program declared, so its
      examples are the first thing a reader of this class needs and the other two build on them.
- [ ] **`Core\Command::run`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/command.rs:127`. Its attack is the one worth writing first: the words
      a launcher wrote are attacker-chosen, so an unknown command, an empty argument list and a
      million-character word all arrive here.
- [ ] **`Core\Command::completions`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/command.rs:136`. The shell script it writes is read by a shell, so its
      example is the one that shows what a user installs.

## Backlog

- `Core\Compress`'s eight features, `crates/nvs-stdlib/src/compress.rs` — the goal's last group,
  and the one whose bench needs real data rather than a repeated byte.
- A styled `Core\Cli\Text` allocates three more times than a plain one, because `of_runs` renders
  the runs into a second string even at `ColorDepth::None`, where that string equals the body
  `built` shares. Memory is priority 5 and this is per call rather than per request, so it is
  recorded here rather than changed — `crates/nvs-stdlib/src/cli.rs:2093`.
