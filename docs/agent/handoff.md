# Handoff

## State

Goal `core-cli-and-2-more`, twelve items landed: `Core\Cli::arguments`, `colorDepth`, `ask`, `confirm`,
`displayWidth`, `escape`, `write`, `isTty`, `width` and `height` before this session, and `select` and
`multiSelect` in it. Each carries `about.md`, three examples with blessed `.out` files, one attack, one
bench and a Rust test marked `covers:`. `python tools/verify.py` is green whole at this tree.

Three features in the group still owe their proofs: `secret`, `live` and `progress`.

**`Core\Test::scriptAnswers` works under `nvs run`, not only under the test runner**, and a prompt it
answers writes no menu and no question. That is what makes an interactive member's example
deterministic: `multiSelect` has no `default`, so its first two examples answer themselves with one
queued line and print the set that came back. A member that has a `default` needs none of it, and
`select`'s three examples take the default, which is what an unattended run does.

`benches/members/core/Cli/multiSelect.nvs` declares `iterations` and `calls 0` and no allocation count.
It measured 29 allocations per round — the menu, the queued answer and the result set together — and no
figure read off the code would have predicted that, so the ledger row is its yardstick rather than a
declaration.

Every `Core\Cli` member reads `perf stale` from `python tools/dossier.py --id`, because the ledger keys
on the implementing file's text (`rule:testing/member-perf-ledger`) and every session of this goal moves
`crates/nvs-stdlib/src/cli.rs`. One `python tools/dossier.py --record-perf --group 'Core\Cli'` at the
end of the goal clears all of them.

## Next group

**Stage: the goal's item list, in file order** — one file set: `crates/nvs-stdlib/src/cli.rs` for the
registry row, the body and the Rust test, plus `docs/examples/core/Cli/<member>/`,
`tests/hostile/core/Cli/<member>/` and `benches/members/core/Cli/<member>.nvs`. One slice is one
feature with all its feature proofs — `rule:testing/feature-proofs`.

- [ ] **`Core\Cli::secret`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:1848`. There is no `default` and the answer is a
      `secret tainted string`, so an example can neither echo it nor let it reach a sink, and an
      unattended call throws `Core\Cli\NotInteractive`. A queued `Core\Test::scriptAnswers` line plus a
      member that accepts a secret is what an example can print.
- [ ] **`Core\Cli::live`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:3084`.
- [ ] **`Core\Cli::progress`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:3229`. This is the item the driver's acceptance check has named
      since session 0030, and closing it closes that check.

## Backlog
- `menu_of` allocates one temporary `String` per menu line (`push_str(&format!(…))`), which is most of
  the 29 allocations a round — `benches/members/core/Cli/multiSelect.nvs`.
- `python tools/dossier.py --record-perf --only A B` measured only the second of two ids in one call —
  `tools/dossier.py`.
- `Core\Cli`'s perf rows are all stale until one `--record-perf --group 'Core\Cli'` at the goal's end —
  `rule:testing/member-perf-ledger`.
