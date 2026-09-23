- **A `.nvst` case's `--RUN--` line is a closed enum in `crates/nvs-test/src/case.rs`, not a command
  line.** Naming a real flag there — `test --list --format=json` — fails the case before it runs, with
  "`--RUN--` is `run`, `test`, … not `…`", however correct the spelling is on the binary. Add the
  spelling as a `Subcommand` variant first: the enum, `args()`, the parser arm, that arm's error
  message and the section table in `crates/nvs-test/src/lib.rs`, which is five edits in two files.
  [until: gone crates/nvs-test/src/case.rs:pub enum Subcommand]
