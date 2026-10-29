---
looper: 1
title: "nvs code renames a name, and lists its callers and what reaches it"
summary: "nvs code rename, calls and reach answer from the editor's own rename, call hierarchy and used-by functions, the rules say so, the agent pointer names them, and the checks below pass."
context: [docs/rules/tooling/nvs-code-answers-about-the-project.md, docs/rules/ide/one-server-two-thin-clients.md, crates/nvs-cli/src/code.rs, crates/nvs-lsp/src/query.rs, crates/nvs-cli/tests/code.rs]
stages:
  - id: 1
    title: "nvs code rename writes the editor's rename to disk"
    status: open
    context: [crates/nvs-lsp/src/rename.rs, docs/rules/ide/no-refactoring-introduces-an-alias.md, crates/nvs-cli/src/fmt.rs]
  - id: 2
    title: "nvs code calls and nvs code reach read the call graph, and the agent pointer names all three"
    status: open
    context: [crates/nvs-lsp/src/calls.rs, crates/nvs-lsp/src/reach.rs, docs/rules/ide/used-by-counts-the-routes-and-tests-that-reach-a-method.md, docs/rules/ide/five-features-are-one-reference-index.md, crates/nvs-cli/src/agent.rs]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: code-rename-tests
    stage: 1
    name: nvs code rename writes every edit the editor's rename makes, and nothing when it refuses
    run: [cargo, test, --test, code]
    tests:
      - code_rename_renames_a_method_at_every_site_and_in_its_overrides
      - code_rename_renames_a_type_and_moves_the_file_named_after_it
      - code_rename_renames_a_local_at_a_position
      - code_rename_refuses_a_clash_and_writes_nothing
      - code_rename_refuses_a_parameter_and_writes_nothing
      - code_rename_of_an_unknown_name_prints_the_nearest
      - code_rename_dry_run_prints_every_edit_and_writes_nothing
      - code_rename_json_lists_every_edit_and_the_moved_file
      - code_rename_refuses_a_file_that_changed_after_it_was_read
  - id: query-rename-tests
    stage: 1
    name: the by-name rename gives the same edits as the editor's rename at the declaration
    run: [cargo, test, --test, query]
    tests:
      - rename_by_name_matches_rename_at_the_declaration
      - rename_by_position_matches_rename_at_that_offset
  - id: code-calls-tests
    stage: 2
    name: nvs code calls and nvs code reach print what the editor's call hierarchy and used-by lens show
    run: [cargo, test, --test, code]
    tests:
      - code_calls_lists_callers_and_callees_grouped_by_method
      - code_calls_includes_a_test_request_to_a_route
      - code_calls_of_an_unknown_name_prints_the_nearest
      - code_reach_lists_the_routes_and_the_tests
      - code_reach_says_at_least_when_a_call_is_not_followed
      - code_calls_and_reach_json_have_every_key
  - id: owners-closes
    stage: 2
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, code-rename-and-calls]
    reach: true
    floor: false
---

# nvs code renames a name, and lists its callers and what reaches it

## Why

Coding agents work without an editor, and they look for commands that answer what an editor answers.
`nvs code` already gives them `show`, `refs`, `impls`, `outline` and `types` from the editor's own
index (`rule:tooling/nvs-code-answers-about-the-project`). Two editor features with the most value to an
agent are still editor-only:

- **Rename.** `crates/nvs-lsp/src/rename.rs` renames a type, a member with its whole override family,
  or a local, and refuses every rename that would miss a site or need an alias. An agent without it
  renames with text search and replace, which is wrong wherever two classes share a method name or a
  class is reached through `autoload`.
- **The call graph.** `crates/nvs-lsp/src/calls.rs` gives a method's callers and callees, including a
  `Core\Test::request` the route table matched, and `crates/nvs-lsp/src/reach.rs` gives the routes and
  the tests that reach a method. An agent uses the second to choose which tests to run after an edit.

The rule's Declined list rejects both, and both reasons are no longer true. It declines `rename`
because "the language server does not rename yet", and it does. It declines call hierarchy because
the editor declines it, and the editor now answers `textDocument/callHierarchy`
(`crates/nvs-lsp/src/capabilities.rs:327`).

Binding agents to the language server itself was considered and is not done. The protocol needs an
`initialize` handshake, an open document and a UTF-16 cursor position, while an agent works with
names, and a shell command is the one tool every agent harness has. `nvs lsp` stays available to a
harness that has its own language-server bridge.

## Standing decisions

**Three verbs, and each one calls one `nvs-lsp` function.** `nvs code` decides nothing about the
language (`rule:ide/one-server-two-thin-clients`):

| Verb | Calls |
|---|---|
| `nvs code rename <name\|path:line:column> <new-name> [--dry-run] [--json]` | `nvs_lsp::query::rename`, a new by-name reader over `rename::rename` |
| `nvs code calls <name> [--json]` | `calls::incoming` and `calls::outgoing` |
| `nvs code reach <name> [--json]` | `reach::reach` |

**`rename` takes a name or a position.** A name is the index's own form, as every other verb takes it.
It resolves to its declaration's site, and the rename runs at that offset. A position is
`path:line:column` in the form the verbs already print (line and column from 1, column in characters).
It is the only way to name a local, because a local has no fully-qualified name. Both forms resolve in
`nvs_lsp::query::rename(workspace, target, new_name) -> Result<Renamed, String>`, so the mapping is
`nvs-lsp`'s and a `.lspt` case or a test can call it. A `$` on the new name is optional, as it is in
the editor.

**`rename` writes by default, and `--dry-run` only prints.** An agent asks for a rename to have it
done. The order is fixed:
1. Compute every edit, and the moved file. A refusal writes nothing.
2. Check that every path is inside the project root, and that the moved file's target is inside it
   too. The path is judged before it is joined, as `docs/agent/coding.md` says. The index holds only
   files under the root, so this check fails only on a bug, and it fails closed.
3. Read each file again and compare its bytes with the bytes the analysis read. If any file differs,
   write nothing and fail, naming the file.
4. Write each file with `fs::write`, as `nvs fmt` does (`crates/nvs-cli/src/fmt.rs:260`), and then
   move the file a type rename moves.

Without `--dry-run` it prints one line per changed file with its number of edits, and one line for a
moved file. With `--dry-run` it prints one line per edit: the position, the old text and the new
text. `--json` gives the edits, the moved file and whether anything was written. A refusal prints
the message `rename.rs` gives and exits 1. With `--json` it is `error`, and `nearest` for an unknown
name, as the other verbs do.

Safe fallback: if a crash between two writes leaves the project half renamed, the next `nvs check`
names every broken site. A journal or a staged write in a temporary directory is not built. `nvs fmt`
writes the same way, and nothing on the request path is involved.

**The refusal message `NOTHING` is worded for both clients.** It says "Put the cursor on", which is
wrong on a command line. It becomes: "There is nothing to rename here. You can rename a class, a
variable, a method, a property, a constant or an enum case." The 32 `.lspt` cases under
`tests/lsp/rename/` that expect the old text are updated in the same slice. This is the intended
change, not an expected output edited to pass. `bun nv peek --locate` or a Grep for "Put the cursor
on a class" lists them.

**`calls` prints both directions in one answer.** The callers come first and then the callees. Each
method is on one line with its kind (`caller` or `callee`), followed by an indented line for each call
site with its position and its source line. One call answers "who uses this and what does it use",
so no `--in` or `--out` flag is added. A name that is not a method fails as an unknown name does,
with the nearest method names.

**`reach` prints the routes, then the tests, and then one `at least` line when `Reach::at_least` is
set.** That line says a call the graph does not follow may reach the method. The JSON has `routes`,
`tests` and `atLeast`.

**Help text and the agent pointer are text an end user reads.** Every new `///` on a verb or a flag
in `crates/nvs-cli/src/code.rs`, and the three new lines in `PROTOCOL`
(`crates/nvs-cli/src/agent.rs:1644`), follow AGENTS.md § *Text an end user reads*.

**The rule changes, and no new rule is written.** `rule:tooling/nvs-code-answers-about-the-project`
gets the three verbs in its body. Its title changes from "reads every file and writes none" to say
that only `rename` writes. Its Declined list loses the `rename` and call-hierarchy entries, and gains
these two:
- a journal or staged write for `rename`, for the reason above
- a `--in` or `--out` flag on `calls`, for the reason above

**Trade-offs.** Security: `rename` writes only inside the project root, and only files whose bytes
have not changed since they were read. No verb runs on the request path. Latency: each call costs one
cold analysis of the project, as the existing verbs do. Memory: one index for the length of one call,
and nothing after it. Simplicity: three verbs over functions the editor already has, and no second
implementation of rename or of the call graph.

**Lanes.** Stage 1 and stage 2 both edit `crates/nvs-cli/src/code.rs`, `crates/nvs-cli/tests/code.rs`
and the rule fragment, so they run in order. The goal depends on no other chain goal.

## Stages

### 1. nvs code rename writes the editor's rename to disk

**Does:** `nvs code rename` renames by name or by position, through the function the editor calls,
and writes the result or prints it.

**Split:** the 32 `.lspt` message updates are one string replaced in each file. A writing subagent
does them while the session writes the Rust. The subagent builds nothing.

- [ ] `nvs_lsp::query::rename`: a name resolves to its declaration's file and start offset, a position resolves to an offset in its file, and both call `rename::rename` with that file's `Analysed` (`analyse_file`, `crates/nvs-lsp/src/document.rs:654`). `crates/nvs-lsp/src/query.rs:301`, `crates/nvs-lsp/src/rename.rs:132`
- [ ] The `NOTHING` message is reworded, and the 32 cases under `tests/lsp/rename/` expect the new text. `crates/nvs-lsp/src/rename.rs:103`
- [ ] The `Rename` verb with `--dry-run` and `--json`, its help text, and the `path:line:column` parser beside `split_lines`. `crates/nvs-cli/src/code.rs:59`, `crates/nvs-cli/src/code.rs:591`
- [ ] The write: root check, unchanged-bytes check, `fs::write`, then the move, in the order the standing decision gives. `crates/nvs-cli/src/code.rs:125`
- [ ] The tests the `query-rename-tests` check names. `crates/nvs-lsp/tests/query.rs:1`
- [ ] The tests the `code-rename-tests` check names. The changed-file test changes a file between two steps through a test hook in `code.rs`, never with a timer. `crates/nvs-cli/tests/code.rs:1`
- [ ] The rule fragment describes `rename`, its title changes, and its Declined list loses `rename` and gains the journal entry. Then `bun nv rules --render`. `docs/rules/tooling/nvs-code-answers-about-the-project.md:57`, `data/rules/tooling/nvs-code-answers-about-the-project.json:2`

### 2. nvs code calls and nvs code reach read the call graph, and the agent pointer names all three

**Does:** an agent gets a method's callers, its callees, and the routes and tests that reach it, and
the pointer `nvs agent init` writes names the three new verbs.

- [ ] The `Calls` verb: both directions in one answer, each site with its position and its line. A name that is not a method prints the nearest method names. `crates/nvs-cli/src/code.rs:59`, `crates/nvs-lsp/src/calls.rs:47`
- [ ] The `Reach` verb: routes, tests and the `at least` line. `crates/nvs-cli/src/code.rs:59`, `crates/nvs-lsp/src/reach.rs:51`
- [ ] The tests the `code-calls-tests` check names. `crates/nvs-cli/tests/code.rs:1`
- [ ] `PROTOCOL` names `rename`, `calls` and `reach`, one line each, and any `agent init` test that pins the text is updated. `crates/nvs-cli/src/agent.rs:1644`
- [ ] The rule fragment describes `calls` and `reach`, and its Declined list loses call hierarchy and gains the `--in`/`--out` entry. Then `bun nv rules --render`. `docs/rules/tooling/nvs-code-answers-about-the-project.md:67`
- [ ] `bun nv render --website` writes the new help into the website's command reference. `website/src/data/reference.json:1195`
- [ ] `bun nv owners --closes code-rename-and-calls` passes. `tools/nv/cmd/owners.ts:1`

### 3. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing is built yet.

### Next

Stage 1, in item order. The first three items share `crates/nvs-lsp/src/rename.rs` and
`crates/nvs-lsp/src/query.rs`, and the rest share `crates/nvs-cli/src/code.rs`.

- [ ] `nvs_lsp::query::rename` by name and by position. `crates/nvs-lsp/src/query.rs:301`
- [ ] The `NOTHING` message and the 32 `.lspt` cases. `crates/nvs-lsp/src/rename.rs:103`
- [ ] The `Rename` verb, the write, and its tests. `crates/nvs-cli/src/code.rs:59`

### Backlog
