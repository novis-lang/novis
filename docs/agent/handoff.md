# Handoff

## State

**Goal `editor-surfaces` — milestone M10. Stage 2's first CLI surface is landed; stage 0 is where it
was.** `nvs check --json` writes one record per diagnostic the text renderer prints:
`crates/nvs-cli/src/check.rs` is the renderer, `Sink` at `crates/nvs-cli/src/main.rs:2096` is what
picks it, and the three tests stage 2's `[[check]]` names pass in `crates/nvs-cli/tests/check.rs`.
The text rendering is untouched and is still the default — `rule:ide/tasks-carry-a-problem-matcher`'s
`problemMatcher` reads it, and stage 3 has not been written yet.

**`tools/verify.py`'s `extension` step is still red on purpose**, the same two contributions tests
as before (`nvs.run`, `nvs.test`, `nvs.showAst`, answered by stages 3 and 4), and the playbook bullet
at `docs/agent/playbook.md:1268` is what says so. Nothing else in the gate is red.

**Stage 2's second surface is wider than its item claimed.** The declaring file and line a Test
Explorer needs are not in `runner.rs` to render: `nvs_types::testing::TestCase` carries no span at
all, so the row has to grow one before the document can print it. `crates/nvs-types/src/testing.rs`
is in the goal's `[context] modules` as of this session for that reason.

## Next group

**Stage 2: the test report, and the ADR that freezes both schemas** — one file set:
`crates/nvs-types/src/testing.rs`, `crates/nvs-cli/src/runner.rs` and `crates/nvs-cli/src/main.rs`.
The first item is what makes the second possible, so they go in this order.

- [ ] **A `#[Test]` row carries where it was declared**, per `rule:testing/test-attribute`'s table:
      `crates/nvs-types/src/testing.rs:417` pushes the row and `crates/nvs-types/src/testing.rs:423`
      pushes the method's own span into `case_spans`, which only diagnostics read. The span is in
      hand at both lines; the struct is `crates/nvs-types/src/testing.rs:243`.
- [ ] **The report to `schemaVersion: 2`, with that file and line on each record**, per
      `rule:testing/report-formats`: `json_document` at `crates/nvs-cli/src/runner.rs:1653` is the
      document and `Case` at `crates/nvs-cli/src/runner.rs:237` is what it renders; the runner reaches
      a `SourceMap` through `crate::Checked`. JUnit and the human format stay byte-identical. Test
      names `every_json_test_record_carries_its_declaring_file_and_line` and
      `the_schema_version_is_two_and_junit_and_human_are_byte_identical`.
- [ ] **A listing mode that discovers without executing**, same rule: a flag on `Command::Test` at
      `crates/nvs-cli/src/main.rs:335`, answered where `runner::run` at
      `crates/nvs-cli/src/runner.rs:258` has the compiled table and before it runs anything. Test
      name `the_listing_mode_discovers_without_executing`. **The open decision is what the listing
      document is** — one `schemaVersion: 2` document whose records carry `class`, `method`, `file`
      and `line` and no verdict, versus a `summary` of zeros that reads as a run where nothing
      passed. Decide it in the ADR below rather than in the emitter.
- [ ] **One ADR**, covering `nvs check --json`'s schema (landed, version 1), the test report's
      version 2 and stage 5's explorer shape — the only record this goal opens. Re-derive the next
      free number from `docs/decisions/` immediately before creating it; it was 0172 at this commit.
      The shape, and the `changes:`/`because` relation it writes twice, is
      `docs/agent/conventions.md:443`.

## Backlog

- The pack prints `loop-goal.md`'s `## Standing decisions` and not the current stage's own prose,
  which is where an item's specification actually is — stage 2's listing mode is named there and
  nowhere else. No `[context]` field selects it; the fix is in `orient.py`, not the manifest.
- Stage 0's two failing contributions tests close in stages 3 and 4 — `docs/agent/loop-goal.md`
  § *Stage 0*.
- `nvs check --json` prints no document when the entry file or the configuration cannot be read;
  both say so on standard error, as `nvs ast --json` does — `crates/nvs-cli/src/main.rs:@run_check`.
