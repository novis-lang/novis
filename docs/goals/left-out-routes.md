---
looper: 1
title: the editor checks a link to a route in a file it leaves out
summary: "Reached when the editor's check builds the route rows of every file it leaves out from that file's declarations, without walking a body, a link to a route in such a file gets the same diagnostics nvs check gives, and the E0754 skip for a left-out file is gone."
context: [crates/nvs-types/src/check.rs, crates/nvs-types/src/links.rs, crates/nvs-lsp/src/session.rs, docs/rules/routing/link-target-and-params-are-checked.md, docs/rules/ide/an-open-document-is-its-own-entry-point.md]
stages:
  - id: 1
    title: "a left-out file's route rows are built from its declarations, and every link to them is checked"
    status: open
    context: [crates/nvs-types/src/routes.rs, crates/nvs-lsp/tests/autoload.rs]
  - id: 2
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: rows
    stage: 1
    name: "a link from the open document to a route in a left-out file gets every link diagnostic nvs check gives, a link to a method there with no route is E0754 again, and no body of a left-out file is walked"
    run: [cargo, test, --test, autoload]
    tests: [a_link_to_a_route_of_a_file_the_analysis_left_out_is_not_an_error, a_link_to_a_route_of_a_left_out_file_is_checked_for_its_keys, a_link_to_a_left_out_method_with_no_route_is_an_error, a_left_out_files_route_rows_walk_no_body, a_left_out_handler_gets_no_helper_read_warning]
    timeout: 1800
  - id: close
    stage: 2
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, left-out-routes]
    timeout: 900
---

# the editor checks a link to a route in a file it leaves out

## Why

The editor checks the open document's bodies alone. Every other file is folded in from the session's
cache where its checked output is current, and left out where it is not
(`crates/nvs-types/src/check.rs:405`, `crates/nvs-lsp/src/session.rs:34`). A left-out file's route
rows are missing from the table. A user report showed the cost: a `Core\Router::url` link to a routed
method of another file was E0754, "that method has no `#[Route]`". `nvs check` gave no error.

Commit `b5a2d79550` stopped the false error. `links::resolve` now skips a link whose handler's class is
declared in a left-out file (`crates/nvs-types/src/links.rs:628`, fed by the closure at
`crates/nvs-types/src/check.rs:670`). So the editor checks nothing about such a link: a key that names
nothing (E0759), a value of the wrong type, and a target with no route all wait for `nvs check`. A file
is left out often, for example whenever the count of anonymous functions before it differs from the
count in the analysis that cached it (`crates/nvs-types/src/check.rs:400`).

A route row is a fact of the declaration. `crate::routes::check_class_routes`
(`crates/nvs-types/src/routes.rs:940`) reads the class's attributes, the method's signature from the
whole-program table (`returns_value`, `routes.rs:2525`), and one input from the body: the request body
reads `note_bodies` kept (`routes.rs:984`). So the rows of a left-out file can be built without its
bodies, and the link check then reads the same table `nvs check` reads.

## Standing decisions

**The declaration pass.** After the walk in `check_files` (`crates/nvs-types/src/check.rs:262`),
every file in `left_out` (filled at `check.rs:417`) gets one pass over its top-level statements. It
follows `check_stmts`' namespace and `use` handling (`check.rs:826`) to build each class's `Ctx`, and
calls `crate::routes::check_class_routes` for each class declaration, and nothing else. It runs with a
scratch `FileOutput` and a scratch `Diagnostics`, as the `[[restrict]]` replacement jobs already do
(`check.rs:383`). Only the scratch output's `routes` are absorbed into the table, and the scratch
diagnostics are dropped: they belong to a file the editor does not show, and that file's own analysis
reports them. Nothing of the pass is stored in the `CheckCache`. If a shared helper is cleaner than a
second namespace walk, the session factors one out of `check_stmts`. Either way, no method body is
walked.

**The skip goes.** With every left-out file's rows in the table, the `left_out` closure in
`links::resolve` has nothing left to guard. It is removed, with `declared_left_out` in `after_walk` and
the `left_out` parameter of `after_walk`. The set itself stays in `check_files`, because the
declaration pass reads it. The paragraph `b5a2d79550` added to `crates/nvs-lsp/src/session.rs:38` is
rewritten to say that a left-out file's route rows come from its declarations, and that the other
whole-table checks still see only the current files.

**The fold stays as it is.** A link to a left-out file's route is recorded as `ExprInfo::RouteLink`,
like any other. Lowering never reads it, because the editor lowers only when nothing was left out
(`crates/nvs-lsp/src/document.rs:1320`). Go to definition and the argument hints read it
(`crates/nvs-lsp/src/definition.rs:337`, `crates/nvs-lsp/src/arguments.rs:202`), and they now work for
these links too.

**The helper-read warning needs no change.** A declaration-only row has no body reads, so
`check_helper_reads` (`routes.rs:2432`) considers it. It then looks for the handler's calls, and those
are recorded only by a walked body, so a left-out handler has none and gets no warning. The test
`a_left_out_handler_gets_no_helper_read_warning` pins this. If it fails, the fallback is a
`declared_only` flag on `Route` that `check_helper_reads` skips.

**Commands stay out.** `check_class_commands` builds the command rows the same way, and a left-out
file's commands are missing from the editor's duplicate-name check. That is a missing warning, never a
false one, so this goal does not touch it.

**Cost.** Nothing at run time, and nothing in `nvs check`, where `left_out` is always empty. In the
editor, one declaration pass per left-out file per analysis. It reads attributes and signatures only,
and it is a no-op for a class with no `#[Route]` (`routes.rs:920`). The test
`a_left_out_files_route_rows_walk_no_body` proves it by a count: the open document's analysis still
checks one file's bodies (`Analysed::work.checked`, `crates/nvs-lsp/src/document.rs:921`).

**Website.** No feature of the language changes. One `website/src/data/changes.json` entry of kind
`fixed` for `tools:editor/nvs-lsp`: a link to a route in another file is no longer reported as a link
to a method without a route, and the editor now checks its keys and values too.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is written to `### Backlog` with what blocked it.

**Lanes.** One content stage, and it runs as a side goal: it depends on no chain goal. It shares
`crates/nvs-types/src/check.rs` with any chain goal that edits the per-class walk, so its branch merges
`main` in before it lands, as every side loop does.

## Stages

### 1. a left-out file's route rows are built from its declarations, and every link to them is checked

- [ ] `crates/nvs-types/src/check.rs:417`: after the walk loop and before `after_walk`, the declaration
  pass of § *The declaration pass* runs over every file in `left_out`, and its rows are absorbed into
  `routes`. `rule:routing/link-target-and-params-are-checked`.
- [ ] `crates/nvs-types/src/links.rs:628` and `crates/nvs-types/src/check.rs:670`: the skip, the
  closure and `after_walk`'s `left_out` parameter are removed. The doc paragraph above `resolve`
  (`links.rs:595`) that explains the skip is removed with them.
- [ ] `crates/nvs-lsp/src/session.rs:38`: the paragraph says what § *The skip goes* says.
- [ ] `crates/nvs-lsp/tests/autoload.rs:253`: the existing test stays green unchanged. Four new tests
  beside it, on the same fixture shape (`Blog`, `Index`, `Pages`): a key that names nothing in a link
  to `Pages::show` is E0759; a link to a method of `Pages` with no route is E0754; the analysis's
  `work.checked` is 1; and a left-out handler that calls a body-reading helper gets no
  `W_BODY_READ_IN_HELPER`. Then `cargo test --test autoload` once.
- [ ] `website/src/data/changes.json:1`: the `fixed` entry of § *Website*, written to AGENTS.md
  § *Text an end user reads*.

### 2. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by the interactive session that fixed the false E0754 in `b5a2d79550`, on 2026-11-25, from a
user report. Nothing of this goal is built. Every anchor above was read on that day.

### Next

Stage 1 from its first item. The check `rows` is red until the four new tests exist.

### Backlog
