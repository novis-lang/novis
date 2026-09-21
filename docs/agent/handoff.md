# Handoff

## State

Goal `core-cli-and-2-more`, six items landed: `Core\Cli::arguments`, `colorDepth` and `ask` before
this session, and `confirm`, `displayWidth` and `escape` in it. Each carries `about.md`, three
examples with blessed `.out` files, one attack, one bench and a Rust test marked `covers:`.
`python tools/verify.py` is green whole at this tree.

Nine features in the group still owe their proofs: `write`, `isTty`, `width`, `height`, `select`,
`multiSelect`, `secret`, `live` and `progress`.

Every `Core\Cli` member still reads `perf stale` from `python tools/dossier.py --id`, because the
ledger keys on the implementing file's text (`rule:testing/member-perf-ledger`) and every session
of this goal moves `crates/nvs-stdlib/src/cli.rs`. One
`python tools/dossier.py --record-perf --group 'Core\Cli'` at the end of the goal clears all of
them; a session taking another item would stale them again.

## Next group

**Stage: the goal's item list, in file order** — one file set: `crates/nvs-stdlib/src/cli.rs` for the
registry row, the body and the Rust test, plus `docs/examples/core/Cli/<member>/`,
`tests/hostile/core/Cli/<member>/` and `benches/members/core/Cli/<member>.nvs`. One slice is one
feature with all its feature proofs — `rule:testing/feature-proofs`.

- [ ] **`Core\Cli::write`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:186`, body at `crates/nvs-stdlib/src/cli.rs:1197`.
      It takes a stream and a style, so an example that prints to `Core\Cli\Stream::Out` is the one
      whose `.out` file is stable.
- [ ] **`Core\Cli::isTty`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:204`, body at `crates/nvs-stdlib/src/cli.rs:1310`.
      With no terminal it answers `false`, and `echo` of a `false` prints nothing at all, so an
      example branches with `if` rather than echoing the value.
- [ ] **`Core\Cli::width` and `Core\Cli::height`** — one shape twice, so they are one slice.
      `crates/nvs-stdlib/src/cli.rs:213` and `crates/nvs-stdlib/src/cli.rs:222`, bodies at
      `crates/nvs-stdlib/src/cli.rs:1319` and `crates/nvs-stdlib/src/cli.rs:1327`. With no terminal
      the profile answers 80 and 24, which is what makes an example's `.out` file stable.

## Backlog

- `Core\Cli::confirm` builds its `[Y/n]` question before it asks, so an unattended call allocates a
  string nobody reads — `crates/nvs-stdlib/src/cli.rs:1634`, against `select`'s `!answerable(ctx)`
  gate at `crates/nvs-stdlib/src/cli.rs:1680`. Measured at 2 allocations a call where `ask` costs 1.
  Left as is: a prompt is human-speed, and a second early exit would split the silence policy
  `unanswered` holds in one place.
- One `python tools/dossier.py --record-perf --group 'Core\Cli'` is owed at the end of this goal.
- `serve::tests::sd_notify_messages_are_ready_then_reloading_and_ready_then_stopping` sent a fifth
  message beside the other test binaries and passed alone — either a shutdown that notifies twice or
  another binary on the same socket, and nobody has read `crates/nvs-cli/src/serve.rs:3913` to say
  which.
- `Core\Cli::select`, `multiSelect`, `secret`, `live` and `progress` are the group's harder half:
  each renders something, so an example's `.out` file is only stable where nothing is watching.
