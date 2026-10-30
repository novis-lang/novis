---
looper: 1
title: "nvs code renames, moves and finds, and lists callers and what reaches a method"
summary: "nvs code rename, move, find, calls and reach answer from the editor's own rename, file move, workspace symbol, call hierarchy and used-by functions, the rules say so, the agent pointer names them, and the checks below pass."
context: [docs/rules/tooling/nvs-code-answers-about-the-project.md, docs/rules/ide/one-server-two-thin-clients.md, crates/nvs-cli/src/code.rs, crates/nvs-lsp/src/query.rs, crates/nvs-cli/tests/code.rs]
stages:
  - id: 1
    title: "nvs code rename writes the editor's rename to disk"
    status: done
    context: [crates/nvs-lsp/src/rename.rs, docs/rules/ide/no-refactoring-introduces-an-alias.md, crates/nvs-cli/src/fmt.rs]
  - id: 2
    title: "nvs code move moves a file or a directory and rewrites every require that names it"
    status: open
    context: [crates/nvs-lsp/src/rename.rs]
  - id: 3
    title: "nvs code find, calls and reach read the index, and the agent pointer names all five verbs"
    status: open
    context: [crates/nvs-lsp/src/calls.rs, crates/nvs-lsp/src/reach.rs, crates/nvs-lsp/src/index.rs, docs/rules/ide/used-by-counts-the-routes-and-tests-that-reach-a-method.md, docs/rules/ide/five-features-are-one-reference-index.md, crates/nvs-cli/src/agent.rs]
  - id: 4
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
  - id: code-move-tests
    stage: 2
    name: nvs code move moves the file or directory and rewrites every require that names it, and nothing when it refuses
    run: [cargo, test, --test, code]
    tests:
      - code_move_moves_a_file_and_rewrites_each_require_of_it
      - code_move_moves_a_directory_and_rewrites_each_require_into_it
      - code_move_keeps_a_relative_require_relative_and_an_absolute_one_absolute
      - code_move_refuses_a_target_that_exists_and_writes_nothing
      - code_move_refuses_a_path_outside_the_project_and_writes_nothing
      - code_move_dry_run_prints_every_edit_and_moves_nothing
      - code_move_json_lists_every_edit_and_the_move
  - id: code-calls-tests
    stage: 3
    name: nvs code find, calls and reach print what the editor's workspace symbols, call hierarchy and used-by lens show
    run: [cargo, test, --test, code]
    tests:
      - code_find_lists_each_declaration_whose_own_name_contains_the_query
      - code_find_ignores_case
      - code_find_with_no_match_prints_nothing_and_succeeds
      - code_calls_lists_callers_and_callees_grouped_by_method
      - code_calls_includes_a_test_request_to_a_route
      - code_calls_of_an_unknown_name_prints_the_nearest
      - code_reach_lists_the_routes_and_the_tests
      - code_reach_says_at_least_when_a_call_is_not_followed
      - code_find_calls_and_reach_json_have_every_key
  - id: owners-closes
    stage: 3
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, code-rename-and-calls]
    reach: true
    floor: false
---

# nvs code renames, moves and finds, and lists callers and what reaches a method

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

Two smaller editor features come with them, because each is one call over a function that exists:

- **A file move.** `rename::moved` (`crates/nvs-lsp/src/rename.rs:210`) rewrites every `require`
  literal that names a moved file or a file under a moved directory, which the editor does on
  `workspace/willRenameFiles`. An agent that moves a file with `mv` breaks each of them.
- **A search for a name.** `SymbolIndex::matching` (`crates/nvs-lsp/src/index.rs:544`) answers
  `workspace/symbol`. `nvs agent find` searches only `Core`, and `nvs code show` needs the full name
  before it answers.

The rule's Declined list rejects both, and both reasons are no longer true. It declines `rename`
because "the language server does not rename yet", and it does. It declines call hierarchy because
the editor declines it, and the editor now answers `textDocument/callHierarchy`
(`crates/nvs-lsp/src/capabilities.rs:327`).

Binding agents to the language server itself was considered and is not done. The protocol needs an
`initialize` handshake, an open document and a UTF-16 cursor position, while an agent works with
names, and a shell command is the one tool every agent harness has. `nvs lsp` stays available to a
harness that has its own language-server bridge.

## Standing decisions

**Five verbs, and each one calls one `nvs-lsp` function.** `nvs code` decides nothing about the
language (`rule:ide/one-server-two-thin-clients`):

| Verb | Calls |
|---|---|
| `nvs code rename <name\|path:line:column> <new-name> [--dry-run] [--json]` | `nvs_lsp::query::rename`, a new by-name reader over `rename::rename` |
| `nvs code move <from> <to> [--dry-run] [--json]` | `rename::moved` |
| `nvs code find <query> [--json]` | `SymbolIndex::matching` |
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

**`move` and `rename` share one write path.** Stage 1 writes it as one function in `code.rs`. It
takes the edits and an optional list of moves, runs steps 2 to 4 above, and prints what it did.
Stage 2 calls it and adds no second writer.

**`move` takes two paths, and does what the editor's file move does.** `<from>` is an existing file
or directory under the project root, and `<to>` is its new path, also under the root. A directory
moves with everything in it, and every indexed `.nvs` file under it counts as moved. `rename::moved`
gives the `require` edits, keyed by each file's current path. So the edits are written first, and
then the file or directory is moved with `fs::rename`. It refuses, and writes nothing, when:
- `<from>` does not exist or is outside the root
- `<to>` already exists, is outside the root, or is inside `<from>`
- `<to>`'s parent directory does not exist

It does not create directories. It also does not rename the type a moved file declares, or change
a namespace an `autoload` maps to a directory. The editor's move does not do either. A type is
renamed with `rename`, which moves its file too. After a move, `nvs check` names any other name that
no longer resolves. Safe fallback: if `fs::rename` fails after the edits are written, the command
fails, names the error, and says which files it already changed.

**`find` prints one line per declaration whose own name contains the query, ignoring case.** That is
the editor's `workspace/symbol` match: a type's last segment, or a member's name without the `$`.
Each line has the position, the kind (as `outline` prints it) and the full name, sorted by name.
No match prints nothing and exits 0, as an empty `refs` does. `Core` names are not searched, because
`nvs agent find` searches them.

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
in `crates/nvs-cli/src/code.rs`, and the five new lines in `PROTOCOL`
(`crates/nvs-cli/src/agent.rs:1644`), follow AGENTS.md § *Text an end user reads*.

**The rule changes, and no new rule is written.** `rule:tooling/nvs-code-answers-about-the-project`
gets the five verbs in its body. Its title changes from "reads every file and writes none" to say
that only `rename` and `move` write. Its Declined list loses the `rename` and call-hierarchy
entries, and gains these four:
- a journal or staged write for `rename` and `move`, for the reason above
- `move` renaming the type a file declares or creating a missing directory, for the reason above
- a `--in` or `--out` flag on `calls`, for the reason above
- `find` searching `Core`, which `nvs agent find` already searches

**Trade-offs.** Security: `rename` and `move` write only inside the project root, and only files whose
bytes have not changed since they were read. No verb runs on the request path. Latency: each call costs
one cold analysis of the project, as the existing verbs do. Memory: one index for the length of one
call, and nothing after it. Simplicity: five verbs over functions the editor already has, one write
path for both writers, and no second implementation of rename, file moves, search or the call graph.

**Lanes.** Stages 1 to 3 all edit `crates/nvs-cli/src/code.rs`, `crates/nvs-cli/tests/code.rs` and
the rule fragment, and stage 2 calls stage 1's write path, so they run in order. The goal depends on
no other chain goal.

## Stages

### 1. nvs code rename writes the editor's rename to disk

**Does:** `nvs code rename` renames by name or by position, through the function the editor calls,
and writes the result or prints it.

**Split:** the 32 `.lspt` message updates are one string replaced in each file. A writing subagent
does them while the session writes the Rust. The subagent builds nothing.

- [x] `nvs_lsp::query::rename`: a name resolves to its declaration's file and start offset, a position resolves to an offset in its file, and both call `rename::rename` with that file's `Analysed` (`analyse_file`, `crates/nvs-lsp/src/document.rs:654`). `crates/nvs-lsp/src/query.rs:301`, `crates/nvs-lsp/src/rename.rs:132`
- [x] The `NOTHING` message is reworded, and the 32 cases under `tests/lsp/rename/` expect the new text. `crates/nvs-lsp/src/rename.rs:103`
- [x] The `Rename` verb with `--dry-run` and `--json`, its help text, and the `path:line:column` parser beside `split_lines`. `crates/nvs-cli/src/code.rs:59`, `crates/nvs-cli/src/code.rs:591`
- [x] The one write path `move` reuses: root check, unchanged-bytes check, `fs::write`, then the moves, in the order the standing decision gives. `crates/nvs-cli/src/code.rs:125`
- [x] The tests the `query-rename-tests` check names. `crates/nvs-lsp/tests/query.rs:1`
- [x] The tests the `code-rename-tests` check names. The changed-file test changes a file between two steps through a test hook in `code.rs`, never with a timer. `crates/nvs-cli/tests/code.rs:1`
- [x] The rule fragment describes `rename`, its title changes, and its Declined list loses `rename` and gains the journal entry. Then `bun nv rules --render`. `docs/rules/tooling/nvs-code-answers-about-the-project.md:57`, `data/rules/tooling/nvs-code-answers-about-the-project.json:2`

### 2. nvs code move moves a file or a directory and rewrites every require that names it

**Does:** an agent moves a file or a directory with one command, and every `require` that named it
still finds it.

- [ ] The `Move` verb with `--dry-run` and `--json`, and its help text. Its checks of `<from>` and `<to>` run before `rename::moved` is called. `crates/nvs-cli/src/code.rs:59`, `crates/nvs-lsp/src/rename.rs:210`
- [ ] It writes through stage 1's write path, then moves with `fs::rename`, and names the changed files when the move fails. `crates/nvs-cli/src/code.rs:125`
- [ ] The tests the `code-move-tests` check names. `crates/nvs-cli/tests/code.rs:1`
- [ ] The rule fragment describes `move`, and its Declined list gains the entry on types and missing directories. `docs/rules/tooling/nvs-code-answers-about-the-project.md:57`

### 3. nvs code find, calls and reach read the index, and the agent pointer names all five verbs

**Does:** an agent finds a declaration by part of its name, and gets a method's callers, its
callees, and the routes and tests that reach it. The pointer `nvs agent init` writes names the five
new verbs.

- [ ] The `Find` verb: one line per match from `SymbolIndex::matching`, with its position, kind and full name. `crates/nvs-cli/src/code.rs:59`, `crates/nvs-lsp/src/index.rs:544`
- [ ] The `Calls` verb: both directions in one answer, each site with its position and its line. A name that is not a method prints the nearest method names. `crates/nvs-cli/src/code.rs:59`, `crates/nvs-lsp/src/calls.rs:47`
- [ ] The `Reach` verb: routes, tests and the `at least` line. `crates/nvs-cli/src/code.rs:59`, `crates/nvs-lsp/src/reach.rs:51`
- [ ] The tests the `code-calls-tests` check names. `crates/nvs-cli/tests/code.rs:1`
- [ ] `PROTOCOL` names `rename`, `move`, `find`, `calls` and `reach`, one line each, and any `agent init` test that pins the text is updated. `crates/nvs-cli/src/agent.rs:1644`
- [ ] The rule fragment describes `find`, `calls` and `reach`, and its Declined list loses call hierarchy and gains the `--in`/`--out` and `Core` entries. Then `bun nv rules --render`. `docs/rules/tooling/nvs-code-answers-about-the-project.md:67`
- [ ] `bun nv render --website` writes the new help into the website's command reference. `website/src/data/reference.json:1195`
- [ ] `bun nv owners --closes code-rename-and-calls` passes. `tools/nv/cmd/owners.ts:1`

### 4. the goal is verified once, and every failure is fixed

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

Stage 1 is done. `nvs_lsp::query::rename` takes a `query::Target` (a name or a position), and
`nvs code rename` writes through `write_out`, the one write path. `planned` applies the edits in
memory and judges every path inside the root. `committed` reads each file again, writes, and moves
last. A debug build reads `NVS_TEST_CODE_CHANGE_BEFORE_WRITE`, which the changed-file test uses.
The JSON is `edits`, `moved` (an array of `{from, to}`, so a directory move fits) and `written`.
`NOTHING` has its new wording, and so do the 32 `.lspt` cases. Both stage 1 checks pass, and so does
clippy. The full `.lspt` suite has not run; stage 4 runs it.

The sweep's reds: `env-secret-proofs` exits 0 when run again (its log is green). Each nightly CI
failure has its fix on main already, after the sha the run tested: the overload job's
`CHANGELOG.md` in ef7e629e2, the passkey Miri ignores in df208a541, and the spool wake test in
dafe6c98d. The next nightly shows whether they hold.

### Next

Stage 2, in item order. Every item is in `crates/nvs-cli/src/code.rs` and its tests.

- [ ] The `Move` verb and its checks of `<from>` and `<to>`, then `rename::moved`. `crates/nvs-cli/src/code.rs:67`, `crates/nvs-lsp/src/rename.rs:211`
- [ ] Write through `write_out` with the move as its one `(from, to)`. `fs::rename` moves a directory as it moves a file. `crates/nvs-cli/src/code.rs:721`, `crates/nvs-cli/src/code.rs:882`
- [ ] The `code-move-tests` tests, beside the rename ones. `crates/nvs-cli/tests/code.rs:640`
- [ ] The rule fragment's `move` line and its Declined entry. `docs/rules/tooling/nvs-code-answers-about-the-project.md:39`

No test should go red. Stage 4 runs the whole `.lspt` suite, which reads the new `NOTHING` text.

### Backlog
