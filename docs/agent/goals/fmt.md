---
milestone: M10
---
# Loop goal 45 — nvs fmt rewrites a file into its one canonical layout

`nvs fmt` exists, and every rule under `tooling/fmt-*` goes from `designed` to `shipped`: one canonical
layout with no configuration, idempotent over the whole corpus, every comment kept where it was, and
inline HTML and markup-literal bodies byte-identical. `nvs fmt <paths>`, `--check`, `--diff` and
`--stdin` are its whole surface, and an editor client has one formatter to start.

[ADR 0039](../../decisions/0039.md) decided the style and [ADR 0173](../../decisions/0173.md) added the
one rule the editor's template pass needs — a `?>` that begins its line sits at its block's depth. This
goal is the implementation.

## Why here

After goal `markup-literal`, because that goal adds the last construct `rule:tooling/fmt-novis-constructs`
lays out — a markup literal, whose body the formatter must leave byte-identical — and a formatter
written before the lexer knows the literal would have to learn it twice.

Before goal `template-format`, which is format-on-save in the editor and whose first step is this
binary: `rule:ide/every-feature-is-staged-behind-its-dependency` makes format-on-save wait for the
formatter. Before goal `gap-zero`, for that goal's standing reason: it can only be emptied once
everything that would add to it has run.

What it needs already built: the lossless parse, `nvs_syntax::parse`
(`crates/nvs-syntax/src/parser/mod.rs:883`) — the same tree `parse_file` builds, with the trivia as a
side channel (goal `resilient-tree`) — and its corpus walk, `crates/nvs-syntax/tests/lossless.rs:140-168`,
which already proves every corpus file is reproduced by its tokens and trivia.

## Stage 1 — the floor

Goal `markup-literal`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: the identity printer

A new crate, `crates/nvs-fmt`, the name `rule:ide/one-server-two-thin-clients` already gives it. It reads
`nvs_syntax::parse` and prints the file back, and the first printer changes nothing: every token and
every trivia item is written as it was read, so over the whole corpus the output is the input. Once that
holds, each style rule is a local change to what the printer writes between two tokens, and a rule that
breaks a file is caught by a test that already walks every file.

A file whose parse reports an error is refused — left as written and named — never formatted.

## Stage 3 — the base style

`rule:tooling/fmt-base-style-is-per`: four spaces per block depth, K&R braces on control structures,
Allman on declarations, one modifier order, PER's blank lines. `rule:tooling/fmt-never-reflows`: an
author's line break inside an expression is kept, and only indentation and the space around tokens are
normalized.

## Stage 4 — the token rules

`rule:tooling/fmt-quotes`, `rule:tooling/fmt-trailing-commas`, `rule:tooling/fmt-sorts-the-use-block` and
`rule:tooling/fmt-normalizes-only-reserved-spellings`, and the two things it never does:
`rule:tooling/fmt-never-inserts-visibility` and `rule:tooling/fmt-never-reorders-members`.

## Stage 5 — Novis's own constructs, and the two modes

`rule:tooling/fmt-novis-constructs`, every bullet. Two are about where the file stops being Novis: an
inline-HTML region and a markup literal's body are byte-identical after formatting, and a `?>` that
begins its line is indented to its block's depth. Only that `?>`'s leading whitespace moves, which is
code and so prints nothing. The markup after it is goal `template-format`'s, in the editor.

## Stage 6 — the command

`nvs fmt <paths>` rewrites in place, `--check` writes nothing and exits non-zero naming each file that
would change, `--diff` prints the diff instead, and `--stdin` reads one file and writes it formatted to
standard output — the form goal `template-format`'s editor half starts. `Command::Fmt` goes beside
`Command::Ast` (`crates/nvs-cli/src/main.rs:195`). No compiler command runs it
(`rule:tooling/fmt-is-never-a-diagnostic`). `crates/nvs-cli/src/service.rs:790` lists `fmt` among the
subcommands the installer refuses; that stays true.

## Stage 7 — the corpus, and the rulebook

`rule:tooling/fmt-is-idempotent`: formatting the corpus twice changes nothing the second time, every
comment survives attached where it started, and every formatted file parses to the same tree as its
input, trivia aside. Then every `tooling/fmt-*` rule flips to `shipped` with its `guardedBy` filled, and
`python tools/rules.py --render`.

## Standing decisions

- **The style is decided; this goal implements it.** ADR 0039 and the fragments under
  `docs/rules/tooling/fmt-*` are the spec. A layout question they do not answer is decided under the
  closest rule's reasoning and written into that fragment, never `BLOCKED`. No configuration file, no flag
  that changes output and no line-width limit — `rule:tooling/fmt-is-one-canonical-style` refuses each.
- **The printer reads `nvs_syntax::parse` and nothing else.** Never `parse_file`, which drops comments,
  and never a second lexer. If the tree lacks something a rule needs, the fix is in `nvs-syntax` and is
  recorded in its module doc.
- **A file with a syntax error is refused, not formatted.** The safe direction: an editor saving a
  half-written file gets it back unchanged. The refusal names the file and exits non-zero, and `--stdin`
  writes nothing to standard output.
- **Never a semantic change.** The formatted file parses to the same tree, and stage 7 holds it. Bytes a
  program prints — inline HTML, a heredoc body, a markup literal's body — are never touched; if a rule
  seems to require touching one, the rule is being misread.
- **Fixtures are pairs, and the expected half is frozen.** `tests/fmt/input/<name>.nvs` beside
  `tests/fmt/formatted/<name>.nvs`, walked by one `nvs-fmt` test. Every file under `formatted/` is a fixed
  point, which is what the command check runs `--check` over.
- **No new ADR.** ADR 0039 and ADR 0173 are the design.
- **What it spends.** A command-line tool: one file's tree and output text at a time, released per file.
  Nothing on the request path and nothing in the runtime.
