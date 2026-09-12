# Handoff

## State

**Goal `fmt` (M10), stage 6 is closed: `nvs fmt` is a subcommand of the `nvs` binary.** Both of the
stage's checks pass — the six named `nvs-cli` tests, and `nvs fmt --check tests/fmt/formatted` over
the frozen half of the fixture tree. Nothing is blocked.

**The command owns which files are read and where the text goes, and no layout rule at all.**
`crates/nvs-cli/src/fmt.rs` resolves the paths (a directory is walked for `.nvs`, a named file is
taken as named), calls `nvs_fmt::format` once per file, and dispatches on a three-variant `Mode`;
`--stdin` is a second entry point beside it, because it resolves no path. A refusal names the file
on standard error and never stops the walk, and nothing is written when the bytes already match.
`--diff` builds its unified diff in that module from a longest-common-subsequence over lines split
on `\n` — `str::lines` would call a file that differs only in its final newline unchanged, which is
the one thing `--check` and `--diff` must never disagree about.

**The fixture pairs are `tests/fmt/input/<name>.nvs` beside `tests/fmt/formatted/<name>.nvs`**,
walked by `crates/nvs-fmt/tests/fixtures.rs`, which holds three things: the two directories name the
same files, each input formats to its pair byte for byte, and each frozen file formats to itself.
Adding a pair needs no Rust.

## Next group

**Stage 7: the corpus, and the rulebook** — one file set: a new `crates/nvs-fmt/tests/corpus.rs`,
one more fixture pair under `tests/fmt/`, and the `docs/rules/tooling/fmt-*.md` fragments.

- [ ] **Formatting the whole corpus twice changes nothing the second time** —
      `rule:tooling/fmt-is-idempotent`. The walk to copy is the one
      `crates/nvs-fmt/tests/identity.rs:49` already makes over `examples/` and `tests/` — including
      the one directory it steps over, `tests/fmt/input`, which is unformatted on purpose — and the
      call is `crates/nvs-fmt/src/lib.rs:184`'s `format`. The test is
      `formatting_the_whole_corpus_twice_changes_nothing_the_second_time`, in a new
      `crates/nvs-fmt/tests/corpus.rs`.
- [ ] **Every formatted corpus file parses to the same tree as its input** — the goal's § *Standing
      decisions*, "never a semantic change". Compare the walk rather than the text:
      `crates/nvs-syntax/src/walk.rs:1` is what `nvs ast` renders a tree from, and a comparison of
      the two renderings is what makes a difference readable. The test is
      `every_formatted_corpus_file_parses_to_the_same_tree_as_its_input`, beside the one above.
- [ ] **A comment in every position the grammar allows survives where it started** —
      `rule:ide/tokens-plus-trivia-reproduce-the-file`. The natural shape is one more fixture pair,
      `tests/fmt/input/comments.nvs` and its frozen half, plus the named test; the pair walker that
      picks it up with no edit is `crates/nvs-fmt/tests/fixtures.rs:66`.
- [ ] **The formatter's rules are shipped** — `python tools/rules.py --show
      tooling/fmt-is-one-canonical-style` must print `shipped`, and it prints `designed` today.
      Flip the `status:` line in `docs/rules/tooling/fmt-is-one-canonical-style.md:2` and in each
      `fmt-*` sibling the goal landed, and say in `crates/nvs-fmt/src/lib.rs:55`'s `# Known gaps`
      what is left.

## Backlog

- A multi-line **enum case list** gains no trailing comma, though an array or argument list does —
  decide whether `rule:tooling/fmt-trailing-commas` covers it, in `crates/nvs-fmt/src/tokens.rs`.
- A **`switch` case label** keeps the column its author wrote it at; `crates/nvs-fmt/src/indent.rs`
  indents the body but not the label, and no rule under `docs/rules/tooling/` decides it.
- `nvs fmt` **writes LF** for a file that arrived as CRLF, and nothing states that it does; the home
  would be `rule:tooling/fmt-is-one-canonical-style` or the crate's module doc.
- `initializes()` in `crates/nvs-cli/src/main.rs:216` has no `Fmt` row, because the formatter
  resolves no configuration tree; its test names the commands it lists one by one.
