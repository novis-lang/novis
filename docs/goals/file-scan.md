---
looper: 1
title: "a folder tree is read while the loop runs, and every entry it returns can be read"
summary: "Reached when Core\\IO::walk reads one folder at a time, depth-first in name order, and gives Core\\IO\\Entry values that know their kind, its skip option and Core\\Path::matches prune with one pattern syntax, list and walk leave out every entry the program may not read, Core\\IO::lines reads its file while the loop runs, and the rules, the feature proofs and the website say so."
context: [crates/nvs-stdlib/src/io.rs, crates/nvs-runtime/src/capability.rs, crates/nvs-stdlib/tests/capability.rs]
stages:
  - id: 1
    title: "list and walk return names in byte order and leave out what the program may not read"
    status: open
    context: [crates/nvs-runtime/src/beneath.rs, docs/rules/config/no-configuration-file-is-a-complete-configuration.md, docs/rules/security/writes-open-beneath-a-handle.md]
  - id: 2
    title: "Core\\Path::matches matches a path against one pattern syntax on every platform"
    status: open
    context: [crates/nvs-stdlib/src/path.rs, crates/nvs-runtime/src/graphemes.rs, docs/rules/core-api/qualifier-behaviour-is-declared.md]
  - id: 3
    title: "walk reads one folder at a time and gives Core\\IO\\Entry values"
    status: open
    context: [crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/instance.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/cursor.rs]
  - id: 4
    title: "walk takes a skip list, and lines reads its file while the loop runs"
    status: open
    context: [crates/nvs-stdlib/src/path.rs, crates/nvs-stdlib/src/str.rs, docs/rules/core-api/options-bag.md]
  - id: 5
    title: "the rules, the reference, the examples, the feature proofs and the website say so"
    status: open
    context: [docs/reference/core/IO.md, docs/reference/core/IO-Walk.md, docs/reference/core/Path.md, website/src/data/changes.json]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: listing
    stage: 1
    name: "list and walk return each folder's names in byte order, leave out every entry the program may not read, and still throw for a start folder it may not read"
    run: [cargo, test, --test, file_scan]
    tests: [a_listing_is_in_byte_order_on_every_platform, a_server_listing_leaves_out_dot_names_and_nvs_toml_under_the_default_grant, a_written_grant_of_the_app_folder_lists_dot_names, a_server_walk_of_the_app_folder_does_not_enter_dot_git, a_start_folder_the_program_may_not_read_still_throws]
    timeout: 1800
  - id: matches
    stage: 2
    name: "Core\\Path::matches follows the pattern syntax of the standing decisions, compares exactly on every platform, and does work bounded by the pattern times the path"
    run: [cargo, test, --test, file_scan]
    tests: [a_pattern_without_a_separator_matches_the_name_at_any_depth, a_pattern_with_a_separator_matches_from_the_start, a_root_or_drive_never_takes_part_in_a_match, double_star_matches_zero_or_more_folders, star_and_question_mark_stay_inside_one_part, brackets_and_braces_match_their_sets, matching_is_exact_and_case_sensitive_on_every_platform, both_separators_split_a_path_and_a_pattern, a_bad_pattern_throws_naming_its_position, a_hostile_pattern_takes_steps_bounded_by_pattern_times_path]
    timeout: 1800
  - id: walk
    stage: 3
    name: "walk gives Core\\IO\\Entry values depth-first in name order, reads one folder per step, holds only the listings along one path, and throws inside the loop for a folder that fails"
    run: [cargo, test, --test, file_scan]
    tests: [a_walk_is_depth_first_in_name_order, an_entry_knows_its_paths_name_and_kind, a_link_is_an_entry_with_is_link_and_is_never_entered, a_break_reads_no_further_folder, a_missing_start_folder_throws_at_the_call, a_folder_that_fails_mid_walk_throws_inside_the_loop, a_second_loop_goes_on_where_the_first_stopped, a_tree_larger_than_the_memory_limit_walks_under_it]
    timeout: 1800
  - id: skip-and-lines
    stage: 4
    name: "skip leaves out matching entries and never opens a matching folder, and lines opens its file at the call, reads it while the loop runs and closes it on every way out"
    run: [cargo, test, --test, file_scan]
    tests: [skip_leaves_out_a_matching_file, skip_never_opens_a_matching_folder, a_bad_skip_pattern_throws_at_the_call, lines_opens_the_file_at_the_call_and_a_missing_file_throws_there, lines_reads_the_file_while_the_loop_runs, lines_closes_the_file_when_the_loop_ends_or_breaks, a_file_larger_than_the_memory_limit_reads_line_by_line_under_it, a_line_that_is_not_utf8_throws_at_that_line]
    timeout: 1800
  - id: proofs
    stage: 5
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, file-scan]
    timeout: 1800
  - id: close
    stage: 6
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, file-scan]
    timeout: 900
---

# a folder tree is read while the loop runs, and every entry it returns can be read

## Why

A program that scans files, such as an uploads folder, a log folder or a project tree, has three
tools today: `Core\IO::list`, `Core\IO::walk` and `Core\IO::lines`. All three read everything when
they are called. `walk` keeps one string per entry of the whole tree (`crates/nvs-stdlib/src/io.rs:4148`),
so a tree with a million files is a million strings before the loop starts. `lines` keeps the whole
file (`io.rs:2444`). Neither can stop early or skip a subtree, and a `walk` entry is a bare string,
so a program pays one more `stat` per entry to learn what the listing already knew
(`crates/nvs-runtime/src/beneath.rs:314` reads the kind, and `io.rs:4161` drops it after the descent
test).

Two more defects. The order of a listing is the operating system's, so one tree walks in a
different order on Windows and Linux. And on a server with no written `fs.read`, the default grant
leaves out every path with a part that starts with `.` (`rule:config/no-configuration-file-is-a-complete-configuration`),
while `walk` asks the door for every folder it enters. So `Core\IO::walk` over the app folder throws
a `RuntimeError` at `.git` with the default configuration: the first call a server program writes to
scan its own folder fails.

The session of 2026-11-26 proposed the five changes below and the two answers on hidden entries and
case. The user agreed to all of them and asked for this goal. Every detail under § *Standing
decisions* beyond those is the session's call, open until the user changes one.

## Standing decisions

**A listing is sorted by name, in byte order.** `list` and every folder `walk` reads sort their
names by the bytes of the UTF-8 name, on every platform (`rule:core-classes/io-a-listing-shows-what-the-program-may-read`,
new). There is no locale and no case folding. The sort runs once per folder in
`crates/nvs-runtime/src/capability.rs:1367` `read_dir`, so every caller gets it. **What it spends:**
O(n log n) name comparisons per folder, beside one `readdir` per folder. The about.md lines that tell
a program to call `Core\Arr::sort` are rewritten.

**A listing leaves out every entry the program may not read.** `read_dir` asks, for each entry, the
question the read door asks (`capability.rs:924` `allow`: the grant, the temporary folders, the
program's own files and `data_folder_refusal`), without throwing, and leaves out each entry it
denies. That covers files and folders, so `.env`, `.git` and `nvs.toml` are not listed under the
server's default grant, and `walk` never opens a denied folder. The start folder itself still goes
through the door and still throws `RuntimeError` when it is denied, so a wrong grant is loud where a
scan starts. Where the grant is every path (a CLI run with no written `fs.read`), the check is not
made at all: a mechanism costs nothing where unused. **What it spends:** one grant comparison per
entry on a scoped grant. Fallback if `allow` cannot answer without building the denial message: split
its yes/no half out as a function both callers use.

**Hidden entries are listed by the grant, never by an option.** A CLI program sees every name. A
server program sees names that start with `.` once its grant covers them, `read = ["."]` under
`[capabilities.fs]`. `rule:config/no-configuration-file-is-a-complete-configuration` says a written
key replaces the default whole, and the test `a_written_grant_of_the_app_folder_lists_dot_names`
proves it. Fallback if a written grant still leaves dot names out: the about.md and the rule name the
grant that does list them, and the test asserts that one. A CLI program that does not want dot names
writes `{ skip: ['.*'] }`.

**`Core\Path::matches(string $pattern, string $path): bool`** is a new static method in
`crates/nvs-stdlib/src/path.rs:104` `CLASS`. It never touches the disk. Its syntax, in
`rule:core-classes/path-matches-is-one-pattern-syntax` (new):
- The path and the pattern are split into parts the way `Core\Path::split` splits a path
  (`path.rs:170`): `/` and `\` are both separators on every platform, and a repeated or trailing
  separator is nothing. A root or drive at the start of the path never takes part in a match.
- A pattern with no separator matches the last part of the path, the name, at any depth: `*.jpg`
  matches `a/b/c.jpg`. A pattern with a separator matches from the first part: `src/*.nvs` matches
  `src/a.nvs` and not `lib/src/a.nvs`. A leading separator only makes a one-part pattern match from
  the start: `/build` matches `build` and not `a/build`. This is the `.gitignore` rule.
- `*` matches any run of characters inside one part, the empty run included. `?` matches one
  character as a person sees it (a grapheme, as `Core\Str` counts one). Neither treats a leading `.`
  specially.
- `**` as a whole part matches zero or more parts. `**` beside other text in a part throws.
- `[abc]`, `[a-z]` and `[!abc]` match one grapheme in or not in the set. A range compares code
  points. `{a,b,c}` matches one of the alternatives. Braces do not nest and do not contain a
  separator. There is no escape character, because both separators are taken. A special character
  is written in brackets: `[*]`, `[?]`, `[[]`, `[{]`.
- The comparison is exact: case-sensitive, and with no Unicode normalization, on every platform.
- A pattern that does not parse throws the class `Core\Regex` throws for a pattern that does not
  compile (`crates/nvs-stdlib/src/regex.rs:1132`), and the message names the position.
- The match is a simulation of every live position at once, never backtracking, so its steps are
  bounded by the pattern's length times the path's length. There is no compile cache. A pattern is
  parsed on each call, and `skip` parses each of its patterns once per walk.
The parameters' qualifiers follow `rule:core-api/qualifier-behaviour-is-declared`. A tainted
pattern is allowed, because the result is a `bool`.

**`Core\IO\Entry`** is a new class, one per entry `walk` gives:
- `path(): string` is the start folder as the program wrote it, joined to the entry with
  `Core\Path::join`'s rules. It is ready for `readText`, `open` or `metadata`.
- `relativePath(): string` is the path relative to the start folder, written with
  `Core\Path::SEPARATOR`, as `walk` gives it today.
- `name(): string` is the last part.
- `isFile()`, `isDirectory()` and `isLink()` come from the listing's `beneath::Kind`
  (`beneath.rs:301`), with no `stat`. A link, or a junction on Windows, is `isLink()` and neither of
  the other two. A device, socket or pipe is none of the three.
Size and modified time are not on it: on Linux they need a `stat` and on Windows the listing has
them, so the cost would differ by platform. A program calls `Core\IO::metadata($entry->path())`.
The strings are not tainted, for the reason `io.rs:318` gives `list`'s. **What it spends:** one
object per entry while the program holds it: the shared start folder, the relative path and the kind.
`path()` builds its string when called.

**`walk` reads while the loop runs.** `Core\IO\Walk` is its own iterator, with `iterate()`,
`advance()` and `current()`, as `Core\Csv\Rows` is (`csv.rs:343`, `instance.rs:990`). The call goes
through the door for the start folder and reads it, so a missing, denied or non-folder start throws
at the call. Each `advance()` gives the next entry. The order is depth-first: an entry, then, if it
is a folder, everything under it, then its next sibling, with siblings in name order. A folder is
read whole, sorted and filtered when the walk reaches it, and its handle is closed before the walk
gives its first entry, so no handle is open between two steps. A folder that fails to read inside
the walk throws `IOError` in the loop, at the step that reaches it, as a folder that cannot be read
to the end throws today. A second loop over the same value goes on where the first stopped, and
after the end it gives nothing, as every `Core` stream does. **What it spends:** the listings of the
folders on the current path, each asked of the request's memory limit as `affords_entry`
(`beneath.rs:58`) asks today. That is O(depth × folder size), where today it is O(tree). A `break`
reads no further folder.

**`walk` takes `{ skip?: array<string> }`.** The signature becomes `walk(string $path, { skip?:
array<string> }): Core\IO\Walk`, with the options row shaped like `READ_TEXT_OPTIONS` (`io.rs:477`)
and `rule:core-api/options-bag`. Each pattern is parsed at the call, so a bad one throws there. An
entry whose `relativePath()` matches any pattern by `Core\Path::matches` is left out, and a matching
folder is never opened. **What it spends:** one match per entry per pattern.

**`lines` reads while the loop runs.** `Core\IO::lines` goes through the door and opens the file at
the call, with the own-file rule `slurp` uses today (`io.rs:2455`), so a missing or denied file
throws at the call. `Core\IO\Lines` becomes its own iterator, like `Walk`. Each `advance()` reads to
the next line ending through one buffer, splits with `crate::str::line_pieces` so a line is what
`Core\Str::lines` says it is, and checks that line is UTF-8. A `\r` at the end of the buffer waits
for the next read before it ends a line, so `\r\n` is one ending. A line that is not UTF-8 throws at
that line. The file closes at the end of the loop, and when the value is freed after a `break` or a
throw. A second loop goes on where the first stopped. The recorded `file` event stays one per call
(`io.rs:2445` `crate::site::watched`). **What it spends:** one open file and one 64 KiB buffer per
value until it closes, and one line at a time, bounded by the request's memory limit as today's
whole file is.

**`list` stays a bare-name `array<string>`.** It reads one folder, and its result is already
bounded by that folder. It gets the sort and the filter and nothing else.

**Off the request core: not in this goal.** Every disk call runs on the core today, and
`rule:http-server/session-expiry-belongs-to-the-store`'s Declined list keeps a local-file call
there until a measured program waits on it. A streamed walk already splits the work into one step
per folder.

**Rules.** Four new fragments under `docs/rules/core-classes/`, with their rows in
`docs/rules/core-classes.json`: `io-a-listing-shows-what-the-program-may-read`,
`io-walk-reads-while-the-loop-runs`, `io-lines-reads-while-the-loop-runs` and
`path-matches-is-one-pattern-syntax`. Each holds its decisions above, its cost and its Declined list.
The "Decision: the entries are held, not streamed" and "Decision: the lines are read and held"
sections of the `WALK` and `LINES` doc comments (`io.rs:1893`, `:1955`) are rewritten whole to name
the new rules. `docs/ground-rules.md` gets its lines from `bun nv rules`.

**Declined, written into the new fragments' Declined lists.**
- **A `hidden` option on `walk` or `list`.** It would list names the program may not read, and on
  Windows "hidden" is a file attribute, not a leading dot. The grant is the switch.
- **Listing names the program may not read, and throwing when it opens one.** The default
  configuration's first scan of the app folder would fail.
- **Following links.** A link can lead out of the granted tree or back up into it.
- **`maxDepth`.** `list` is depth one, and deeper limits are rare. It would be a second way to do
  what `list` does.
- **Size and modified time on `Core\IO\Entry`.** Their cost differs by platform.
- **Walking folders in parallel inside `walk`.** `Core\Task::map` with `limit` over the entries is
  the way.
- **Case-insensitive matching, now.** A `{ ignoreCase: true }` option on `matches` can be added
  later without breaking anything, if programs need it. Today `*.{jpg,JPG}` covers a camera's names.
- **Unicode normalization in `matches`.** The comparison is exact, like the case.
- **A folder-only pattern with a trailing separator.** `matches` does not touch the disk, so it
  cannot know what is a folder.
- **An escape character.** Both separators are taken. Brackets cover every special character.
- **Skipping a folder that vanished during a walk.** It would hide a real change. The walk throws,
  as it does today.

**Feature proofs.** New features: `Core\Path::matches` and every member of `Core\IO\Entry`. Changed:
`Core\IO::walk`, `Core\IO::list` and `Core\IO::lines`. `bun nv proofs --owed --goal file-scan` names
them and every other feature the work reached. The examples use neutral names. `matches`: match a
file name, prune a folder with `**`, and a set in brackets and braces. `Entry`: one per member from
the same small tree, such as a photo folder. `walk`: a tree in order, `skip` for `node_modules` and
`.git`, and the total size of the `.jpg` files with `metadata`. `lines`: the existing three, with a
fourth only if the proofs ask. The attacks: a pattern built to backtrack, such as many `*` and a long
name; a tree wider and deeper than the memory limit walked with a `break`; a server program that
tries to list `.env` and `.git` under the default grant; a file larger than the memory limit read
with `lines`. The benches: `benches/members/core/IO/walk.nvs` records its new counts, a new
`benches/members/core/Path/matches.nvs` states its growth in the pattern and the path, and
`benches/members/core/IO/lines.nvs` records its new counts.

**Website.** `website/src/data/changes.json` entries: `new` `Core\Path::matches`, `new`
`Core\IO\Entry`, `changed` `Core\IO::walk` (streamed, ordered, entries, `skip`), `changed`
`Core\IO::list` (ordered, leaves out what may not be read) and `changed` `Core\IO::lines`
(streamed). Each is written to AGENTS.md § *Text an end user reads*.

**What stops it.** One attempt per item and two sessions per stage, counted in `### State`. An item
that cannot land in one attempt is written to `### Backlog` with what blocked it. No item is added
to a stage.

**Lanes.** Stage 2 touches only `crates/nvs-stdlib/src/path.rs` and the new test file's own tests, so
it can run as a side goal beside stage 1. Stages 1, 3 and 4 share `io.rs` and `capability.rs` and run
in order, and stage 4 needs stage 2's matcher. The goal runs on the chain after `narrowed-static`.
`zip-writer` also edits `io.rs` and `registry.rs`, so the two do not run side by side.

## Stages

### 1. list and walk return names in byte order and leave out what the program may not read

- [ ] `crates/nvs-runtime/src/capability.rs:1367`: `read_dir` sorts the entries by the bytes of the
  name and leaves out each entry the read door would deny, with no check where the grant is every
  path. Its doc comment says so, with what it spends. `rule:core-classes/io-a-listing-shows-what-the-program-may-read`.
- [ ] `crates/nvs-runtime/src/capability.rs:924`: the yes/no half of `allow` is callable without
  building a denial, per § *A listing leaves out*.
- [ ] `crates/nvs-stdlib/src/io.rs:4116` and `:4148`: the doc comments of `list` and `walk` say
  "sorted" and "left out", and nothing else in their bodies changes in this stage.
- [ ] `crates/nvs-stdlib/tests/file_scan.rs:1` (new, its contexts built like `ctx_with` in
  `crates/nvs-stdlib/tests/capability.rs:150`; the server default grant needs the server run kind,
  found where `nvs_config` writes that default): the five tests of the check `listing`, then
  `cargo test --test file_scan` once.
- [ ] Tests that assert the old order are updated: `io.rs:5648` and `:5716`, and the cases
  `tests/conformance/core/io-list-hands-back-bare-names-and-neither-of-scandirs-two.nvst` and
  `io-walk-answers-the-tree-where-list-answers-one-directory.nvst` if their expected order changes.

### 2. Core\Path::matches matches a path against one pattern syntax on every platform

- [ ] `crates/nvs-stdlib/src/path.rs:104`: the `matches` row, its `MethodDoc` with end-user text and
  its helper, per § *`Core\Path::matches`*. The matcher is a set-of-positions simulation over the
  parts `split` gives (`path.rs:170`), with graphemes from `nvs_runtime::graphemes`. It returns its step
  count to the test that bounds it. `rule:core-classes/path-matches-is-one-pattern-syntax`.
- [ ] The capability and taint tables that list every `Core` member name it, as `split`'s rows do
  (the session finds them by `nvs_core_path_split`).
- [ ] The ten tests of the check `matches` in `crates/nvs-stdlib/tests/file_scan.rs`, then
  `cargo test --test file_scan` once.

### 3. walk reads one folder at a time and gives Core\IO\Entry values

- [ ] `crates/nvs-stdlib/src/io.rs:1940`: `WALK` becomes its own iterator with the trio's symbols
  and slots for the start folder and the stack of sorted listings. `ENTRY` (`Core\IO\Entry`) is a
  new class with its six members and end-user docs. The `WALK` doc comment is rewritten whole.
  `rule:core-classes/io-walk-reads-while-the-loop-runs`.
- [ ] `crates/nvs-stdlib/src/io.rs:4148` and `:4189`: `nvs_core_io_walk` reads the start folder
  through the door, and `iterate`, `advance` and `current` walk depth-first, reading each folder
  when the walk reaches it, as § *`walk` reads while the loop runs* says.
- [ ] `crates/nvs-stdlib/src/io.rs:331`: the row's return comment. `crates/nvs-stdlib/src/registry.rs:4469`:
  the element type of `WALK_NAME` is `CoreTy::Instance` of `Core\IO\Entry`. `registry.rs:7775`
  `HANDLES`, `crates/nvs-stdlib/src/instance.rs:312` and `:884` (the trio, as `:990` lists `Csv\Rows`'),
  and `crates/nvs-stdlib/src/classes.rs:33` name the new class.
- [ ] Every caller of `walk` gives up its string entries: `docs/examples/core/IO/walk/01-walk-a-folder-tree.nvs`,
  `02-empty-folders-and-files.nvs` and `03-size-of-an-uploads-folder.nvs`, `benches/members/core/IO/walk.nvs`
  and `walk.scale.nvs`, `tests/hostile/core/IO/walk/01-deep-wide-and-bad-trees.nvs`,
  `docs/reference/core/IO-Walk.md`, and the three `tests/conformance/core/io-walk-*.nvst` cases. The
  case `io-walk-is-a-snapshot-that-can-be-taken-twice-and-a-directory-is-an-entry.nvst` is renamed
  for the new rule that a second loop goes on where the first stopped, because the rule changed.
  The Rust test `io.rs:5716` asserts the depth-first order.
- [ ] The eight tests of the check `walk` in `crates/nvs-stdlib/tests/file_scan.rs`. The link test
  makes a symbolic link on Unix and a junction on Windows, and runs in WSL with `bun nv linux-test`
  too. Then `cargo test --test file_scan` once.

### 4. walk takes a skip list, and lines reads its file while the loop runs

- [ ] `crates/nvs-stdlib/src/io.rs:331` and `:477`: the `walk` row takes `CoreTy::Options` with
  `skip`, and `nvs_core_io_walk` parses each pattern at the call with stage 2's matcher and leaves
  out each match without opening it.
- [ ] `crates/nvs-stdlib/src/io.rs:1875` and `:2444`: `LINES` becomes its own iterator over an open
  file and one buffer, per § *`lines` reads while the loop runs*. `nvs_core_io_lines_iterate`
  (`:2479`) and `held_strings` (`:2502`) go once nothing reads them. The `LINES` doc comment is
  rewritten whole. `rule:core-classes/io-lines-reads-while-the-loop-runs`.
- [ ] Tests that assert the old reading are updated: `io.rs:4630` (a bad byte now throws at its
  line) and `:5987`, and the cases `tests/conformance/core/io-lines-walks-a-file-forward-and-walks-it-again.nvst`
  and `io-lines-asks-the-door-read-asks-before-it-splits-anything.nvst`, each renamed for the new rule.
- [ ] The eight tests of the check `skip-and-lines` in `crates/nvs-stdlib/tests/file_scan.rs`. The
  `skip_never_opens_a_matching_folder` folder has more entries than the test's memory limit, so
  opening it would throw on every platform. Then `cargo test --test file_scan` once.

### 5. the rules, the reference, the examples, the feature proofs and the website say so

**Split:** once the four rule fragments are written, the examples, attacks and benches go to two
writing subagents with disjoint folders: one takes `docs/examples/core/Path/matches/`,
`docs/examples/core/IO-Entry/` and their attack and bench; the other takes `docs/examples/core/IO/walk/`,
`list/`, `lines/` and theirs. Each brief names AGENTS.md § *Text an end user reads* and
§ *Feature proofs* above. Neither builds.

- [ ] `docs/rules/core-classes/` four new fragments and their rows in `docs/rules/core-classes.json`,
  per § *Rules*, then `bun nv rules`.
- [ ] `docs/examples/core/IO/walk/about.md`, `list/about.md` and `lines/about.md` are rewritten for
  the new behaviour, including how a server lists dot names. `docs/reference/core/IO.md`,
  `IO-Walk.md`, `IO-Lines.md` and `Path.md` are updated, and `docs/reference/core/IO-Entry.md` is
  new, shaped like `IO-Walk.md`.
- [ ] The other examples that call `list` or `lines` are rerun, and each whose output changed with
  the order is regenerated: `docs/examples/core/IO-Metadata/isDirectory/03-print-a-folder-tree.nvs`,
  `IO-Metadata/isFile/03-copy-only-the-files.nvs`, `IO-Metadata/modifiedAt/03-find-the-newest-backup.nvs`,
  `IO/copy/03-a-dated-backup-of-each-report.nvs`, `IO/isDirectory/03-count-the-files-in-every-folder.nvs`,
  `IO/isFile/03-read-only-the-files-in-a-folder.nvs`, `IO/metadata/03-list-a-folder-with-sizes.nvs`,
  `IO/remove/03-clean-up-unfinished-downloads.nvs` and `IO/removeDirectory/03-delete-a-folder-and-everything-in-it.nvs`.
  An example that sorted only to fix the order drops that sort.
- [ ] The feature proofs `bun nv proofs --owed --goal file-scan` names: about.md, three examples,
  the bench, the attack, the Novis and Rust tests and the help text for each.
- [ ] `website/src/data/changes.json:1`: the five entries of § *Website*, then `bun nv site --check stale`,
  `changes` and `review` green.
- [ ] `bun nv proofs --owed --goal file-scan` names nothing.

### 6. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by an interactive session on 2026-11-26 after the user agreed to the proposal. Nothing is
built. Every anchor was read on that day. One claim is read from the rule and not yet run: that a
written `read = ["."]` lists dot names on a server. Stage 1's test settles it.

### Next

Stage 1 from its first item. Stage 2 can run beside it as a side goal. The check `listing` is red
until `crates/nvs-stdlib/tests/file_scan.rs` exists with its five tests.

### Backlog
