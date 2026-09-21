# Handoff

## State

Goal `core-cli-progress-and-6-more`: three of its sixteen features are done —
`Core\Cli\Progress::advance`, `Core\Cli\Style::of` and `Core\Cli\Text::plain` each carry an
`about.md`, three examples with blessed `.out` files, one attack, one bench with a judged
`allocations` declaration, and one Rust `#[test]` with its `covers:` marker. `python
tools/dossier.py --id '<feature>'` says `complete.` for all three.

Nothing is blocked. The remaining thirteen items are `Core\Cli\Text`'s other two members in
`cli.rs`, then `Core\Command` in `command.rs` and `Core\Compress` in `compress.rs`.

## Next group

**`Core\Cli\Text`'s two remaining members** — one file set, the same one this session held:
`crates/nvs-stdlib/src/cli.rs`, `docs/examples/core/Cli-Text/`, `tests/hostile/core/Cli-Text/`,
`benches/members/core/Cli-Text/`. Both are `rule:testing/feature-proofs`, and the examples for
`Core\Cli\Text::plain` beside them are the model for what an example in this tree looks like.

- [ ] **`Core\Cli\Text::styled`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:1961`. `Core\Cli\Style::of`'s `about.md` describes what a style
      is, so this one describes putting one on text; its Rust test has room at the claim no `.nvst`
      case reaches, that a styled run renders as the SGR its `Style` asks for at each `ColorDepth`,
      since a case runs at `ColorDepth::None` where every style renders as nothing.
- [ ] **`Core\Cli\Text::text`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/cli.rs:1971`. The instance member that reads a `Text` back as a
      `string` without its styling; `rendered_at(slot(receiver, TEXT_RUNS), ColorDepth::None)` at
      `crates/nvs-stdlib/src/cli.rs:2265` is the whole body.

## Backlog

- `benches/members/lang/types/void-never-self-static.nvs` declares `allocations 2` and measures 1,
  so its figure is not recorded; one other bench was in the same state in that run —
  `benches/members/README.md` owns what a declaration means.
- Items 6 to 8 of the goal are `Core\Command` in `crates/nvs-stdlib/src/command.rs`, a different
  file set — `docs/agent/loop-goal.md` § *The item list*.
- Items 9 to 16 are `Core\Compress` in `crates/nvs-stdlib/src/compress.rs`, a third file set.
