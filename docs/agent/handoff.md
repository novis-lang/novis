# Handoff

## State

**Goal `fmt` (M10), stage 5 is closed: every construct PER never saw has its one layout.** The
qualifier's space, the `fn` closure's body brace, the one-line object literal's braces and now the
`match` arm list are all landed, and the stage's other check — a close tag at its block's depth with
markup byte-identical — has been green since the session before. Nothing is blocked.

**A `match` arm list is laid out by two halves, because a depth and a line break are different
edits.** `crates/nvs-fmt/src/indent.rs`'s `arm_starts` reads the arm boundaries off the `Match`'s
children — the walk gives an arm no node of its own, so the child after a body opens an arm and the
`,` or `{` in front of it is where a `default` arm's keyword is found — and `of_line` places an arm
that already opens a line, plus the brace that closes the list. `crates/nvs-fmt/src/space.rs`'s
`arm_lines` contributes the `Runs` pair that gives an arm sharing a line with the one before it a
line of its own, and only that case: a run written over an arm that already opens a line would take
the blank line its author left above it too. `Match` has left `indent.rs`'s `OPAQUE` as a result, so
a block written inside an arm is indented like any other now.

The corpus absorbed this rule with no edit — `examples/match.nvs` was already what the rule asks for.

## Next group

**Stage 6: the command** — one file set: `crates/nvs-cli/src/main.rs`, a new
`crates/nvs-cli/src/fmt.rs`, a new `crates/nvs-cli/tests/fmt.rs`, and the fixture pairs under
`tests/fmt/`.

- [ ] **`nvs fmt <paths>` rewrites each named file in place, and refuses one that does not parse** —
      `rule:tooling/fmt-is-one-canonical-style`. The subcommand is a variant in
      `crates/nvs-cli/src/main.rs:228`'s `enum Command` with a module beside `crates/nvs-cli/src/check.rs:1`
      to copy the shape from, and the one call it makes is `crates/nvs-fmt/src/lib.rs:184`'s `format`.
      The test is `fmt_rewrites_each_named_file_in_place` in a new `crates/nvs-cli/tests/fmt.rs`.
- [ ] **`--check`, `--diff` and `--stdin` are I/O modes and never style knobs** —
      `rule:tooling/fmt-is-one-canonical-style` names the flag set. `--check` writes nothing and exits
      non-zero naming each file, `--diff` prints the diff and writes nothing, `--stdin` writes the
      formatted file to standard output and writes *nothing at all* for a file that does not parse
      (`crates/nvs-fmt/src/lib.rs:191`'s `Refusal`). Four tests, one per mode, in
      `crates/nvs-cli/tests/fmt.rs`.
- [ ] **No compiler command formats anything** — `rule:tooling/fmt-is-never-a-diagnostic`. The test is
      `nvs_check_never_reports_an_unformatted_file` over `crates/nvs-cli/src/check.rs:1`, which must not
      gain a call into `nvs_fmt`.
- [ ] **Every file under `tests/fmt/formatted/` is a fixed point** — the goal's § *Standing decisions*
      pairs `tests/fmt/input/<name>.nvs` with `tests/fmt/formatted/<name>.nvs`, walked by one `nvs-fmt`
      test; neither directory exists yet. The stage's command check runs `nvs fmt --check` over the
      frozen half (`docs/agent/loop-goal.toml:8886`).

## Backlog

- A multi-line object literal's field lines keep the author's indentation — `ObjectLiteral` is in no
  body list; `crates/nvs-fmt/src/lib.rs`'s known gap 2 is where it would be named.
- A shape type's braces get neither the space nor the trailing comma — `crates/nvs-fmt/src/lib.rs`
  known gap 5, and it needs a node in `crates/nvs-syntax/src/walk.rs` first.
- Stage 7 is the corpus and the rulebook — `docs/agent/loop-goal.toml`, stage `7 the corpus`.
