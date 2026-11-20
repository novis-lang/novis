---
looper: 1
title: "a mutation run is aimed, drops what no test can kill, and lists only survivors worth reading"
summary: "Reached when nvs test --mutate takes a path, a class or a function and #[NoMutate], nvs test --since <ref> runs only the tests a change can reach and mutates only the changed lines, nvs test <file>:<line> runs the test at that line, a mutant that compiles to the same code as the program or as an earlier mutant runs no test, judged survivors are read from mutants.toml, log and exception-message sites make no mutant, survivors are grouped by function, a run compiles the program once for all its mutants, and the CLI reference names every flag of nvs test."
context: [crates/nvs-cli/src/runner/mutation.rs, crates/nvs-ir/src/lower/mutate.rs, docs/rules/testing/mutation-testing.md]
stages:
  - id: 1
    title: "every mutant knows its function, and a run is aimed by a path, a name or #[NoMutate]"
    status: done
    context: [crates/nvs-cli/src/runner.rs, crates/nvs-types/src/testing.rs, crates/nvs-types/src/expr_table.rs, crates/nvs-ir/src/lower/mod.rs, docs/rules/testing/report-formats.md, docs/rules/testing/tests-never-reach-a-build.md]
  - id: 2
    title: "--since <ref> reads the changed lines from git, and a mutation run mutates only those"
    status: done
    context: [crates/nvs-cli/src/runner.rs, crates/nvs-cli/tests/test_command.rs]
  - id: 3
    title: "--since selects the tests a change can reach, and <file>:<line> runs the test at that line"
    status: done
    context: [crates/nvs-cli/src/runner.rs, docs/rules/testing/a-run-can-stop-early-rerun-failures-and-shuffle.md, docs/rules/testing/tags-and-shards-select-what-runs.md, crates/nvs-cli/tests/test_command.rs]
  - id: 4
    title: a mutant that compiles to the program's code, or to an earlier mutant's, runs no test
    status: open
    context: [crates/nvs-codegen/src/lib.rs, crates/nvs-cli/src/cache.rs, docs/rules/testing/report-formats.md]
  - id: 5
    title: judged survivors are read from mutants.toml, glue makes no mutant, and survivors are grouped by function
    status: open
    context: [crates/nvs-cli/src/runner.rs, crates/nvs-cli/tests/test_command.rs]
  - id: 6
    title: a mutation run compiles the program once, with every mutant behind one switch
    status: open
    context: [crates/nvs-codegen/src/lib.rs, docs/rules/testing/a-debugged-program-is-compiled-with-a-debug-switch.md, tools/nv/cmd/scaling.ts, benches/scaling/testing/mutate.nvs]
  - id: 7
    title: the CLI reference names every flag of nvs test, and the pages say what this goal added
    status: open
    context: [docs/reference/tools/10-cli.md, docs/examples/tools/cli/nvs-test-mutate/about.md, docs/examples/tools/cli/nvs-test/about.md, docs/agent/prose.md]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: aim
    stage: 1
    name: "a survey names the function of each mutant, --mutate=<target> mutates only the named folder, file, class or method, and #[NoMutate] keeps a class or method out"
    run: [cargo, test, --test, test_command, --, nvs_test_mutate_aimed_at_a_folder_mutates_only_its_files, nvs_test_mutate_aimed_at_a_method_mutates_only_its_body, nvs_test_mutate_skips_a_class_or_method_with_no_mutate, nvs_test_mutate_aimed_at_nothing_that_exists_exits_with_an_error]
    timeout: 1800
    tests:
      - nvs_test_mutate_aimed_at_a_folder_mutates_only_its_files
      - nvs_test_mutate_aimed_at_a_method_mutates_only_its_body
      - nvs_test_mutate_skips_a_class_or_method_with_no_mutate
      - nvs_test_mutate_aimed_at_nothing_that_exists_exits_with_an_error
  - id: aim-ir
    stage: 1
    name: "a survey names the function every mutant is in, an anonymous function and a generator included"
    run: [cargo, test, --lib, --, a_survey_names_the_function_each_mutant_is_in]
    timeout: 1800
    tests:
      - a_survey_names_the_function_each_mutant_is_in
  - id: since-mutate
    stage: 2
    name: "--since mutates only lines changed or untracked since the ref, refuses a ref git does not know, and never reads a ref as a git option"
    run: [cargo, test, --test, test_command, --, nvs_test_mutate_since_mutates_only_changed_and_untracked_lines, nvs_test_since_a_ref_git_does_not_know_exits_with_an_error, nvs_test_since_a_ref_that_starts_with_a_dash_is_not_a_git_option, nvs_test_since_outside_a_checkout_exits_with_an_error]
    timeout: 1800
    tests:
      - nvs_test_mutate_since_mutates_only_changed_and_untracked_lines
      - nvs_test_since_a_ref_git_does_not_know_exits_with_an_error
      - nvs_test_since_a_ref_that_starts_with_a_dash_is_not_a_git_option
      - nvs_test_since_outside_a_checkout_exits_with_an_error
  - id: since-select
    stage: 3
    name: "--since runs the tests that reached a changed function, every new or edited test, and every test when a declaration, a non-.nvs file or the map itself says it cannot tell"
    run: [cargo, test, --test, test_command, --, nvs_test_since_runs_only_the_tests_that_reached_a_changed_function, nvs_test_since_runs_a_new_or_edited_test, nvs_test_since_runs_every_test_when_a_declaration_changed, nvs_test_since_runs_every_test_when_a_file_that_is_not_nvs_changed, nvs_test_since_runs_every_test_with_no_stored_map, nvs_test_since_counts_a_file_edited_since_the_map_as_changed]
    timeout: 1800
    tests:
      - nvs_test_since_runs_only_the_tests_that_reached_a_changed_function
      - nvs_test_since_runs_a_new_or_edited_test
      - nvs_test_since_runs_every_test_when_a_declaration_changed
      - nvs_test_since_runs_every_test_when_a_file_that_is_not_nvs_changed
      - nvs_test_since_runs_every_test_with_no_stored_map
      - nvs_test_since_counts_a_file_edited_since_the_map_as_changed
  - id: at-line
    stage: 3
    name: "<file>:<line> runs the test whose method holds the line, a class line runs its class, and a line outside test code exits with an error"
    run: [cargo, test, --test, test_command, --, nvs_test_at_a_line_runs_the_test_that_holds_it, nvs_test_at_a_class_line_runs_its_class, nvs_test_at_a_line_outside_test_code_exits_with_an_error, nvs_test_at_a_line_keeps_a_windows_drive_letter]
    timeout: 1800
    tests:
      - nvs_test_at_a_line_runs_the_test_that_holds_it
      - nvs_test_at_a_class_line_runs_its_class
      - nvs_test_at_a_line_outside_test_code_exits_with_an_error
      - nvs_test_at_a_line_keeps_a_windows_drive_letter
  - id: equivalent
    stage: 4
    name: "a mutant whose function compiles to the program's code is equivalent and runs no test, and of two mutants that compile alike the second is a duplicate of the first"
    run: [cargo, test, --test, test_command, --, nvs_test_mutate_drops_a_mutant_that_compiles_to_the_programs_code, nvs_test_mutate_runs_one_of_two_mutants_that_compile_alike]
    timeout: 1800
    tests:
      - nvs_test_mutate_drops_a_mutant_that_compiles_to_the_programs_code
      - nvs_test_mutate_runs_one_of_two_mutants_that_compile_alike
  - id: equivalent-codegen
    stage: 4
    name: "a function compiled alone gives the same bytes twice, and a changed comparison gives different ones"
    run: [cargo, test, --lib, --, a_function_compiled_alone_gives_the_same_bytes_twice, a_function_with_a_changed_comparison_gives_different_bytes]
    timeout: 1800
    tests:
      - a_function_compiled_alone_gives_the_same_bytes_twice
      - a_function_with_a_changed_comparison_gives_different_bytes
  - id: judged
    stage: 5
    name: "mutants.toml's judged survivors are counted and not listed, an entry that matches no mutant is named, a key survives an edit above its function, glue makes no mutant, and survivors are grouped by function"
    run: [cargo, test, --test, test_command, --, nvs_test_mutate_reads_judged_survivors_from_mutants_toml, nvs_test_mutate_names_a_judged_entry_that_matches_no_mutant, nvs_test_mutate_refuses_a_judged_entry_with_no_reason, nvs_test_mutate_groups_survivors_by_function_with_one_line_per_source_line]
    timeout: 1800
    tests:
      - nvs_test_mutate_reads_judged_survivors_from_mutants_toml
      - nvs_test_mutate_names_a_judged_entry_that_matches_no_mutant
      - nvs_test_mutate_refuses_a_judged_entry_with_no_reason
      - nvs_test_mutate_groups_survivors_by_function_with_one_line_per_source_line
  - id: judged-unit
    stage: 5
    name: "a log call or an exception message makes no mutant"
    run: [cargo, test, --lib, --, a_log_call_and_an_exception_message_make_no_mutant]
    timeout: 1800
    tests:
      - a_log_call_and_an_exception_message_make_no_mutant
  - id: once
    stage: 6
    name: "one mutation run compiles the program once, each mutant changes only its own site, and the verdicts are the ones a compile per mutant gave"
    run: [cargo, test, --test, test_command, --, nvs_test_mutate_compiles_the_program_once_for_every_mutant, nvs_test_mutate_two_mutants_in_one_function_each_change_only_their_own_site, nvs_test_mutate_reports_each_mutant_and_fails_only_on_a_failing_test, nvs_test_mutate_reverses_an_equality_unless_it_narrows_a_nullable]
    timeout: 1800
    tests:
      - nvs_test_mutate_compiles_the_program_once_for_every_mutant
      - nvs_test_mutate_two_mutants_in_one_function_each_change_only_their_own_site
      - nvs_test_mutate_reports_each_mutant_and_fails_only_on_a_failing_test
      - nvs_test_mutate_reverses_an_equality_unless_it_narrows_a_nullable
---

# a mutation run is aimed, drops what no test can kill, and lists only survivors worth reading

## Why

`nvs test --mutate` mutates the whole program, compiles all of it once per mutant, and lists every
survivor. On a real project that is about an hour, and most of the survivors are noise: glue code
that any test or no test catches, equivalent mutants that no test can kill, and code that will be
rewritten before anybody reads the report. Mutation testing pays off on code that is mostly
decisions — parsers, query building, calculations, limits and permissions — where a wrong `<` or
`&&` gives a wrong result without crashing. Today the run cannot be pointed at that code: `--filter`
narrows the tests, never the mutants (`crates/nvs-cli/src/runner/mutation.rs:95` `list` returns every
site).

The same conversation found two gaps in `nvs test` itself. A failure prints `file:line`, and that
form does not select a test. And nothing runs only the tests a change can reach, which is the
command a developer runs most often. The diff that aims a mutation run is the diff that selects
those tests, so both use one flag. The CLI reference also lists 8 of `nvs test`'s flags
(`docs/reference/tools/10-cli.md:221`); `--tag`, `--skip-tag`, `--shard`, `--fail-fast`,
`--rerun-failed`, `--shuffle`, `--jobs` and `--show-output` are on no page.

## Standing decisions

**The scope (the person's call).** All six changes: aiming, mutating only changed lines, dropping
equivalent and duplicate mutants, a file of judged survivors, skipping glue and grouping survivors,
and one compile per run. Plus `--since` for test selection, `<file>:<line>`, and the reference page.
There is still no mutation gate on any goal: the person decided that before this goal, and nothing
here reopens it. Other test-runner flags were considered and left out: see § *Declined*.

**A mutant names its function (my call).** `nvs_ir::lower::Mutant` (`crates/nvs-ir/src/lower/mutate.rs:77`)
gains `function: String`, the label of the function the site is lowered in, as
`crate::lower::lower_method` names it (`Class::method`). A site in an anonymous function or a
generator body takes the label of the method it is written in, so a user finds it by the name they
wrote. It costs one string per mutant for the length of a run.

**The key of a mutant (my call).** `{function}#{n}: {change}`, where `n` counts the mutants of that
same `function` and `change` in source order, from 1. Example: `Shop\Cart::total#2: \`<\` to \`<=\``.
An edit outside the function does not change it, and the line number is not part of it.
`mutants.toml` and `--format json` use it. A rename of the function changes the key, and the old
entry is then reported as matching no mutant.

**Aiming (my call).** `--mutate` alone mutates everything, as now. `--mutate=<target>` takes one or
more comma-separated targets, with `require_equals` so a path after it is never read as a target:

| Target | Mutates |
|---|---|
| a folder or a file, relative to the current folder | every site in those files |
| `Shop\Cart` | every site in that class's methods |
| `Shop\Cart::total` | every site in that method |

A target that matches no file, class or method exits with status 2 and names it. The filter applies
to `mutation::list`'s result before anything is lowered again, so an aimed run compiles only the
mutants it keeps.

**`#[NoMutate]` (my call).** A `Core\Test` attribute on a class or a method, checked in
`crates/nvs-types/src/testing.rs` beside `#[Test]` and `#[Bench]` (`:1286`). It needs `because:`, a
non-empty reason, as `skip:` does. A site under it makes no mutant, the way test code makes none
(`crate::lower::mutate::within`, called at `crates/nvs-ir/src/lower/mod.rs:1070`). It has no
effect outside `--mutate`, and a normal build pays nothing. There is no opposite attribute:
`--mutate=<target>` and `--since` aim a run, and one attribute is simpler than two.

**`--since <ref>` (my call).** One flag of `nvs test`, for a program's tests only; beside a `.nvst`
tree it exits with an error, as the coverage flags do. `nvs` runs
`git diff --no-ext-diff --no-color --unified=0 --end-of-options <ref> --` and
`git ls-files --others --exclude-standard` in the program's folder, as a child process with no
shell. The changed lines are the added and modified lines of the working tree against `<ref>`, and
every line of an untracked file. `--end-of-options` keeps a ref that starts with `-` from being read
as a git option; a ref that starts with `-` is also refused before git runs, because no branch,
tag or commit is named that way. No `git` on `PATH`, a folder outside a checkout and a ref git does
not know each exit with status 2 and say which. `--since` never falls back to running everything
silently: an error is clearer than a slow run that looks normal. `nvs` gains no git library. It
runs only on a developer's machine or CI, never on the request path.

**`--since` with `--mutate` (my call).** A mutant is kept when any line of its span is a changed
line. It combines with `--mutate=<target>`: both must hold.

**What `--since` runs, as built (my call).** Four git commands, not two: `rev-parse
--show-toplevel` for the root every path is relative to and the "not a checkout" error,
`rev-parse --verify --quiet --end-of-options <ref>^{commit}` for the "unknown ref" error, then the
diff and `ls-files -z --others --exclude-standard --full-name`. The diff also passes
`--no-textconv --no-relative --src-prefix=a/ --dst-prefix=b/`, so a user's `diff.*` settings cannot
move a path. The dash is refused by clap's value parser (`runner::since_ref`) and again in
`since::changed`. A line that was only deleted adds no range; stage 3 decides how a deletion inside
a function marks it changed. `--since` has `requires = "mutate"` until stage 3 gives it a meaning
alone, and stage 3 removes that. An attack on `nvs test`'s arguments is a `// proof: test-refused
<args>` line (`tools/nv/proofs/run.ts` `TEST_REFUSED_RE`): it passes on status 2 with an `error:`
line, and `tests/hostile/README.md` lists it.

**`--since` without `--mutate`: test selection (my call).** A run that measures per-test coverage
writes a *reached map* to the data folder, beside the failed-tests file
(`crates/nvs-cli/src/runner.rs:1126` `failed_tests_file`), keyed the same way by the entry path.
It holds, per test, the labels of the functions it reached, and per program file the blake3 hash
of its content at that run. A coverage flag, `--mutate` and `--since` itself write it; a run with
none of them does not, and pays nothing. Since `--since` writes the map, every `--since` run
measures coverage, so its tests run with the coverage probes on: say that in the help text.

A function is *changed* when a changed line is in its span, or when its file's hash differs from the
one in the map. The second rule keeps the map honest when it was written from a working tree that
has changed since. A test runs when any of these hold:

- it reached a changed function;
- it is new, or its own method or its class's hooks are changed;
- it is not in the map.

Every selected test runs, with no narrowing, when any of these hold, and the run prints one line
saying why:

- there is no map for this entry;
- a changed line is in a declaration rather than a body: a class, interface or enum header, a
  property, a constant, a method signature, an attribute or a `use`. A new override changes which
  code a call reaches, on lines no test reached before;
- a changed file is not a `.nvs` file, such as `nvs.toml` or a data file a test reads.

The rule says plainly that `--since` is for the edit-and-run loop, and that CI runs the whole suite.
A data file that was edited and then put back since the map was written is not seen. `--since`
combines with `--filter`, `--tag`, `--skip-tag` and `--shard`: a test must pass all of them. Beside
`--rerun-failed` it exits with an error, because the two choose tests by different evidence.

What the map spends: on disk, one line per test with its function labels, and one hash per program
file, per entry path. It grows with the program, never with runs, because every run replaces it.

**`<file>:<line>` (my call).** A path argument that does not exist, and ends in `:` and digits, is
read as a file and a line when the part before the colon exists. So `C:\app\CartTest.nvs:42` keeps
its drive letter. A line inside a `#[Test]` method runs that test. A line in a test class outside
its methods runs the whole class. A line outside test code exits with status 2 and names the
nearest test class in the file. It selects like `--filter` does, so it combines with the other
selectors. It is one form of a path, not a new flag.

**Equivalent and duplicate mutants (my call).** This is Trivial Compiler Equivalence: a mutant whose
changed function compiles to the same machine code as the original is equivalent, and two mutants
whose function compiles alike are duplicates. Before any test runs, each reached mutant's changed
function is compiled alone, at the run's `nvs_codegen::opt_level`, with probes off. Its bytes and
its relocation targets, by name, are compared with the original function's and with the earlier
mutants' of the same function. The compare uses a blake3 hash of the bytes and the relocations, so
the run keeps one hash per mutant. Two new outcomes:

| Outcome | When |
|---|---|
| `equivalent` | the changed function compiles to the original's code |
| `duplicate` | it compiles to an earlier mutant's code; the record names that mutant's key |

Neither runs a test and neither counts in the score. A duplicate shows its first mutant's outcome
in the human report's grouping. The rule says honestly that Cranelift folds less than an optimising
C compiler, so the share it finds is whatever it is: stage 4 measures it on the bench member and
writes the figure in the rule. Until stage 6, the changed function is taken from `lower_mutant`'s
program; after stage 6, from the same lowering with the one mutant fixed on.

**`mutants.toml` (my call).** An optional file in the program's folder (the folder `nvs.toml` is
in, `snapshot.folder()`; with no project, the entry file's folder):

```toml
[[judged]]
mutant = "Shop\\Cart::total#2: `<` to `<=`"
reason = "the loop ends on the same item either way"
equivalent = true
```

`reason` must be non-empty, as `skip:` must be. `equivalent` defaults to `false`. A survivor whose
key has an entry is `judged`: the human report counts it and does not list it. A judged survivor
that is `equivalent` is left out of the score, like an `equivalent` mutant. Any other judged
survivor still counts as survived, so the score stays a measurement. An entry that matches no
mutant of this run is printed as a warning with its key, and changes no exit status, so the file
does not rot. A malformed file exits with status 2. No command writes the file: a person judges
each survivor, and the human report prints each survivor's key ready to paste. It is only read
under `--mutate`.

**Glue (my call).** A site whose nearest enclosing call is a `Core\Log` method, or the argument list
of the constructor in a `throw new`, makes no mutant. These change a message, never a result, and
Google's mutation work calls them *arid nodes*. Output written with `echo` or `Core\Html` is not
glue: in a web language the page is the product. A user who wants a view class left out writes
`#[NoMutate]` on it. The list is enumerated where it is implemented, beside the operators, so
adding a case changes no rule.

**Grouping survivors (my call).** The human report lists survivors grouped by function. Functions
are ordered by their number of survivors, most first, then by name. Within a function, one line
per source line: the first survivor's change, and `(+N more on this line)`. `--format json` keeps
every record.

**One compile per run: mutant schemata (my call).** The lowering arms every kept, reached mutant at
once. Each site lowers to its original and its changed form behind one test of a per-run *mutant
number*: `if (active == k) changed else original`. The number is read with one load from the task's
context, and the run sets it before each mutant's suite. It is one compiler with a switch, like
`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`: only a `--mutate` run sets it,
a unit compiled with it is never written to the artifact cache, and a normal build and a server pay
nothing. A site's two forms keep every constraint the survey already holds them to, so the
narrowing argument at `crates/nvs-ir/src/lower/mutate.rs:18` still holds. The per-mutant
whole-program compile is removed, so there is one path. What it spends: a mutation unit is bigger
by one changed copy of each armed expression plus one load and branch per site, and a test under it
runs that much slower; the unit lives for the run. The rule's Declined entry *Recompiling only the
mutated function* stays: this is one compile, not a second recompile path.

**Report format (my call).** `--format json` under `--mutate` becomes `schemaVersion: 5` in stage 1,
and the version stays 5 for the rest of the goal, which lands as one release. Version 5 is version
4 plus, on each mutant record, `function` and `key`; the outcomes `equivalent`, `duplicate` (with
`sameAs`, the first mutant's key) and the `judged` flag on a survived record (with its `reason`).
`docs/rules/testing/report-formats.md:25` says so.

**The rules (my call).** `docs/rules/testing/mutation-testing.md` is rewritten stage by stage, each
stage adding what it built. `--since` and `<file>:<line>` are selectors, so they go in
`docs/rules/testing/tags-and-shards-select-what-runs.md`, retitled to name all four ways a test is
selected. The reached map's storage goes in
`docs/rules/testing/a-run-can-stop-early-rerun-failures-and-shuffle.md` beside the failed-tests
file. `bun nv rules --render` and `bun nv rules --check` after each.

**The reached map, as built (my call).** It hashes each method body (`function <hash> <label>`),
each test class outside its bodies (`class`), and each program file outside its test classes and
bodies (`file`). The class and file hashes read whitespace runs as one space. One hash per whole
file would mark every method of an edited file changed, so `--since` would select by file. A body
hash that differs marks its method changed. A class hash that differs runs its class. A file hash
that differs, or a file missing from the map, runs every test. A blank added line outside a body is
ignored. Each added line is judged alone, because git aligns a hunk across two methods. A deletion
point `p` sits between lines `p` and `p + 1`, and is in a body only when both are. A changed line in
a test method runs that test; elsewhere in a test class, the whole class. A narrowed run keeps the
stored entries of the tests it did not run, while the program still has them. Under `--mutate`,
`--since` narrows mutants only and every test runs. `<file>:<line>` needs the file to exist, and
only one path may name a line. All of it is in `crates/nvs-cli/src/runner/reach.rs`.

**Declined (my call, written into the rules' Declined lists).**
- *Regular expressions or globs in `--filter`.* A substring, tags and shards already select every
  set a suite has needed, and each syntax is one more thing to learn.
- *A coverage minimum flag.* CI reads the coverage files, and a threshold there is one line of CI.
- *A slowest-tests flag.* `--format json` has `durationMs` per test.
- *`#[Mutate]` as an opt-in.* `--mutate=<target>` and `--since` aim a run without editing the code.
- *A command that writes `mutants.toml`.* Each entry is a judgment, and a command would accept a
  survivor nobody read.
- *A git library.* The `git` program is on every machine that has a checkout.

**What it costs.** Nothing for a run without `--mutate` or `--since`, and nothing on the request
path. `--since` runs with coverage probes on, and the reached map is written per entry path. A
mutation run gains one function compile and one hash per reached mutant, loses all but one
whole-program compile, and its unit carries every armed site.

**Lanes.** Every stage but 7 touches `crates/nvs-cli/src/runner.rs` or
`crates/nvs-cli/src/runner/mutation.rs`, so stages 1 to 6 run in order. Stage 7 touches only pages,
and may run as a side goal once stage 5 is done; its sentence about what a run compiles waits for
stage 6.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage
still open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. every mutant knows its function, and a run is aimed by a path, a name or #[NoMutate]

**Does:** the survey records each mutant's function, `--mutate` takes targets, and `#[NoMutate]`
keeps code out. The JSON report becomes version 5 with `function` and `key`.

- [x] `crates/nvs-ir/src/lower/mutate.rs:77` `Mutant` gains `function`, filled at `:343`
  `mutation_site` from the lowering's own function label. A site in an anonymous function or
  generator takes its method's label. `a_survey_names_the_function_each_mutant_is_in` beside the
  test at `:600`, and the `aim-ir` check run once.
- [x] `crates/nvs-types/src/testing.rs:1286`: `#[NoMutate]` with `because:`, checked like
  `#[Bench]`'s roster, and `crates/nvs-types/src/expr_table.rs:2620` gains `is_no_mutate(label)` for
  a method or its class. `crates/nvs-ir/src/lower/mod.rs:1070` passes it to `mutate::within` beside
  `test_code`. The `Core\Test` reference card says what the attribute does, in end-user prose.
- [x] `crates/nvs-cli/src/runner.rs:321` `mutate: bool` becomes an optional list with
  `require_equals` and `num_args(0..=1)` and a comma delimiter. Its help text is rewritten whole in
  end-user prose. `crates/nvs-cli/src/runner/mutation.rs:95` `list` filters by the targets, and a
  target that matches nothing exits with status 2 at `crates/nvs-cli/src/runner.rs:1056`.
- [x] `crates/nvs-cli/src/runner/mutation.rs:68` `Scored` gains `function` and `key`, and
  `crates/nvs-cli/src/runner.rs:4604` `json_document` writes version 5.
  `docs/rules/testing/report-formats.md:25` says what version 5 adds.
- [x] `docs/rules/testing/mutation-testing.md`: the function, the key, aiming and `#[NoMutate]`.
- [x] The four `aim` tests in `crates/nvs-cli/tests/test_command.rs`, beside `:399`, with the
  `// covers: tools:cli/nvs-test-mutate` line. The `aim` check runs once.

### 2. --since <ref> reads the changed lines from git, and a mutation run mutates only those

**Does:** one module that turns a ref into changed lines, and the mutation filter that reads it.
Test selection waits for stage 3.

- [x] `crates/nvs-cli/src/runner/since.rs:1`: the module. It runs the two git commands, parses
  `--unified=0` hunks into a map from canonical path to changed line ranges, adds untracked files
  whole, refuses a ref that starts with `-`, and gives the three errors. Its module doc states what
  it spends: one range list per changed file for the run.
- [x] `crates/nvs-cli/src/runner.rs:156`: the `--since <ref>` flag beside the selectors, refused
  beside a `.nvst` tree. Help text in end-user prose.
- [x] `crates/nvs-cli/src/runner/mutation.rs:95`: under `--since`, keep a mutant whose span has a
  changed line, together with any targets.
- [x] `docs/rules/testing/mutation-testing.md`: `--since`, the commands it runs, `--end-of-options`,
  and the three errors.
- [x] The four `since-mutate` tests in `crates/nvs-cli/tests/test_command.rs`. Each makes its own
  git repository under its scratch folder, sets `user.name`, `user.email` and
  `commit.gpgsign=false` with `-c`, and never reads the global git configuration
  (`GIT_CONFIG_GLOBAL` set to an empty file). The dash case passes `--since=-o/tmp/x` and shows that
  no file is written and the run exits with status 2. The `since-mutate` check runs once.
- [x] `tests/hostile/tools/cli/nvs-test/02-a-ref-that-is-a-git-option.nvs:1`: an attack whose
  `// proof:` line runs `test --since=--output=x`. If `tests/hostile/README.md` gives a `proof:`
  line no checkout, it shows the refusal of the dash before git runs, which needs none.

### 3. --since selects the tests a change can reach, and <file>:<line> runs the test at that line

**Does:** the reached map, the selection with its fail-safe rules, and the line form of a path.

- [x] `crates/nvs-cli/src/runner.rs:1126`: `reached_map_file(entry)` beside `failed_tests_file`,
  with a write after a run that measured per-test coverage (beside `:1153` `record_failed_tests`)
  and a read. The format is text: one `test <Class::method>` line followed by its function labels,
  then one `file <hash> <path>` line per program file. A write that fails is a warning.
- [x] `crates/nvs-cli/src/runner.rs:1580` `Selection` gains the `--since` selection, computed from
  the map, the changed lines from stage 2 and the checked program's spans: changed functions, the
  declaration rule, the non-`.nvs` rule, a new or edited test. The one line that says why every
  test runs is printed under `human`, and goes to standard error under the machine formats.
- [x] `crates/nvs-cli/src/runner.rs:858`: `--since` turns `measured` on, so its run writes a fresh
  map.
- [x] `crates/nvs-cli/src/runner.rs:156`: a path ending in `:` and digits, as the standing decision
  says, selects the test or class at that line through `Selection`.
- [x] `docs/rules/testing/tags-and-shards-select-what-runs.md`: retitled, with `--since` and
  `<file>:<line>`. `docs/rules/testing/a-run-can-stop-early-rerun-failures-and-shuffle.md`: the
  reached map beside the failed-tests file, and `--since` refused beside `--rerun-failed`.
- [x] The six `since-select` and four `at-line` tests in `crates/nvs-cli/tests/test_command.rs`,
  with `// covers: tools:cli/nvs-test`. Both checks run once.

### 4. a mutant that compiles to the program's code, or to an earlier mutant's, runs no test

**Does:** the equivalence filter before any mutant's tests run, and the two new outcomes.

- [ ] `crates/nvs-codegen/src/lib.rs:2689`: a public `function_code_hash(program, name)` that
  compiles one function alone into a throwaway module at `opt_level()`, with probes off, and
  returns the blake3 hash of its bytes and its relocation targets by name. The two
  `equivalent-codegen` tests in that crate's tests, and the check run once.
- [ ] `crates/nvs-cli/src/runner/mutation.rs:112` `score`: before a reached mutant runs, hash its
  changed function, compare it with the original function's (hashed once per function) and with
  earlier mutants' of that function, and give `equivalent` or `duplicate`. `Verdict` (`:44`) gains
  both, and `report` (`:273`) prints their counts and leaves them out of the score.
- [ ] `crates/nvs-cli/src/runner.rs:4604`: the two outcomes and `sameAs` in version 5.
- [ ] The two `equivalent` tests. The duplicate fixture is `if ($a == $b)`: the equality negation
  and the branch negation likely compile alike. The equivalent fixture is one the stage finds that
  Cranelift folds; if none exists at the run's level, the test uses a duplicate's first mutant
  against a hand-written equivalent function and the rule says that no operator was found that
  Cranelift folds. The `equivalent` check runs once.
- [ ] Record on `benches/scaling/testing/mutate.nvs`'s program how many mutants were equivalent and
  duplicate, and write the share into `docs/rules/testing/mutation-testing.md` with the rest of
  this stage's rule text.

### 5. judged survivors are read from mutants.toml, glue makes no mutant, and survivors are grouped by function

**Does:** the file of judgments, the glue sites, and the report's grouping.

- [ ] `crates/nvs-cli/src/runner/mutation.rs`: read `mutants.toml` from the program's folder with
  the `toml` crate the workspace already uses, check each entry's `reason`, mark judged survivors,
  and warn for entries that match no mutant. A malformed file exits with status 2.
- [ ] `crates/nvs-ir/src/lower/mutate.rs:330` `mutation_site`: a site whose nearest enclosing call
  is a `Core\Log` method, or the argument list of a `throw new` constructor, makes no mutant. The
  lowering tracks this the way `in_test_code` is tracked (`:187`). `a_log_call_and_an_exception_message_make_no_mutant`
  beside `:600`, and the `judged-unit` check run once.
- [ ] `crates/nvs-cli/src/runner/mutation.rs:273` `report`: survivors grouped by function, most
  survivors first, one line per source line with `(+N more on this line)`, and each line ends with
  the key to paste into `mutants.toml`. The judged count is printed beside the other counts.
- [ ] `docs/rules/testing/mutation-testing.md`: `mutants.toml`, glue and the grouping.
- [ ] The four `judged` tests in `crates/nvs-cli/tests/test_command.rs`, and the check run once.

### 6. a mutation run compiles the program once, with every mutant behind one switch

**Does:** mutant schemata. One lowering arms every kept, reached mutant; the run sets the active
mutant number before each suite.

- [ ] `crates/nvs-ir/src/lower/mutate.rs:179` `Mode` gains `Schema(Vec<(Span, Operator, u32)>)`.
  An armed site lowers both forms behind a read of the active mutant number, and `lower_mutant`
  (`:140`) is replaced by `lower_schema`. The module doc's *Two passes* and *What it spends*
  sections are rewritten.
- [ ] `crates/nvs-codegen/src/lib.rs`: the active mutant number lives in the task's context, read
  with one load, and the unit is marked so `crates/nvs-cli/src/cache.rs:1698` `unit_for` never
  caches it, as a debug unit is not.
- [ ] `crates/nvs-cli/src/runner/mutation.rs:112` `score`: one compile, then set the number and run
  each mutant's tests. Stage 4's function hash takes its changed function from a lowering with that
  one mutant fixed on.
- [ ] `nvs_test_mutate_compiles_the_program_once_for_every_mutant` reads the count of compiles. If
  the run has no count to read, the human report's mutant note at
  `crates/nvs-cli/src/runner.rs:1057` says `1 compile`, and the test reads that.
  `nvs_test_mutate_two_mutants_in_one_function_each_change_only_their_own_site` checks that two
  mutants in one function get their own verdicts. The two existing tests in the `once` check must
  stay green unchanged. The check runs once.
- [ ] `benches/scaling/testing/mutate.nvs`: a second member, `mutate-program.nvs`, whose program
  grows around a fixed suite, `scaling: kind mutate` and `scaling: expect linear`. Today's
  whole-program compile per mutant would make it grow with the square. Record it with the command
  `benches/scaling/README.md` names.
- [ ] `docs/rules/testing/mutation-testing.md`: *What it costs* rewritten for one compile, with the
  bench's measured growth, and the Declined list keeps *Recompiling only the mutated function*.

### 7. the CLI reference names every flag of nvs test, and the pages say what this goal added

**Does:** the user pages. Every line is text an end user reads, held to `AGENTS.md` § *Text an end
user reads*. An `Explore` subagent first lists every flag `crates/nvs-cli/src/runner.rs:150-470`
declares, with its help text's first line, and the session writes the pages from that list.

- [ ] `docs/reference/tools/10-cli.md:221`: the synopsis names every flag of `nvs test`, and the
  list below it has one entry per flag. Each entry says what the flag does, in one or two sentences,
  and links to the section that says more where one exists. `<file>:<line>` and `--since` are among
  them.
- [ ] `docs/reference/tools/10-cli.md:334`: the `nvs test --mutate` section covers targets,
  `#[NoMutate]`, `--since`, the two new outcomes, `mutants.toml` with the example above, the
  grouping, and one compile per run. `:399` no longer says each mutant compiles the program again.
- [ ] `docs/examples/tools/cli/nvs-test-mutate/about.md:1` and
  `docs/examples/tools/cli/nvs-test/about.md:1`: one short paragraph each for what this goal added,
  within the about.md length bounds.
- [ ] `docs/examples/tools/cli/nvs-test-mutate/04-no-mutate-on-a-view.nvs:1` with its `.out`: a
  class with `#[NoMutate(because: ...)]` makes no mutant, and the report shows it. `bun nv proofs
  --id 'tools:cli/nvs-test-mutate'` and `--id 'tools:cli/nvs-test'` owe nothing.
- [ ] `bun nv proofs --comments` over every `.nvs` this stage wrote.

### 8. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and this goal's own tests at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done; `aim`, `aim-ir` and `since-mutate` are green. Stage 1: `Mutant.function`,
`#[NoMutate]` (`crates/nvs-types/src/derive.rs` `NO_MUTATE`, `E0880`), `mutation::Aim::resolve`,
keys numbered before the aim filters, JSON version 5.

Stage 3 is done; `since-select` and `at-line` are green, each run once. The decision *The reached
map, as built* lists what differs from the plan: hashes per method body and per test class, not one
per file. `crates/nvs-cli/src/runner/reach.rs` has `Layout`, the map (`map_file`, `read`, `record`),
`select`, `at_line`, `split_line` and `Pick`. `Selection` gained `line` and `picks`. `Flags` gained
`reached_map`, which `main.rs` `run_program_pass` sets. `since::Lines` is now `Whole`, `Ranges {
added, deleted }` or `Gone`. `Folder::reached_tests` is `reached/` in the data folder. `--since` no
longer needs `--mutate`.

Stage 2: `since::changed(folder, ref)` in `crates/nvs-cli/src/runner/since.rs`; a mutation run
calls `Aim::since`. The tests' git runs under `test_command.rs` `git_env`, because the scratch
folder is inside this repository's checkout. Nightly run 44510687786 was on adc7487822, and later
commits fixed each of its failures.

### Next

Stage 4: equivalent and duplicate mutants. Start at `crates/nvs-codegen/src/lib.rs`, around the
anchor stage 4's first item names, for `function_code_hash`. Then `crates/nvs-cli/src/runner/mutation.rs`
`score`: hash each reached mutant's changed function before its tests run. The outcomes go in
the JSON document beside `json_document`'s mutant records in `crates/nvs-cli/src/runner.rs`.

Tests expected red until the gate: the CLI reference's flag check, if one exists, may now name
`--since`; stage 7 writes it. No `changes.json` entry yet: it comes with the goal's last stages.

### Backlog
