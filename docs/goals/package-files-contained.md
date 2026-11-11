---
looper: 1
title: a fetched package stays in its folder and in its namespace
summary: "Reached when no reader of a package path accepts an empty, dot or dot-dot part, a git tree entry is one path component, a package carries one package.toml and a manifest with no name ends no namespace check, the registry build compares namespaces exactly, nvs test and nvs check walk into no package below the folder they were given, and nvs test runs no test of a package class the program loads."
context: [crates/nvs-cli/src/package/pack.rs, docs/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.md, docs/rules/packaging/a-package-file-stays-under-its-declared-prefix.md]
stages:
  - id: 1
    title: one function judges every package path
    status: done
    context: [crates/nvs-cli/src/package/git.rs, crates/nvs-cli/src/package/fetch.rs, crates/nvs-cli/src/package/download.rs, tests/hostile/README.md]
  - id: 2
    title: "a package carries one package.toml, its own"
    status: done
    context: [crates/nvs-cli/src/package/fetch.rs, crates/nvs-cli/src/package/download.rs, crates/nvs-cli/tests/package_fetch.rs, docs/rules/packaging/a-package-cannot-reach-its-host.md]
  - id: 3
    title: "the outermost package decides a file's namespace, and the registry build compares exactly"
    status: open
    context: [crates/nvs-types/src/package.rs, crates/nvs-cli/tests/package_grants.rs, crates/nvs-cli/src/package/build.rs, docs/rules/security/an-unmatched-namespace-holds-the-application.md]
  - id: 4
    title: nvs test and nvs check walk into no package below the folder they were given
    status: open
    context: [crates/nvs-test/src/lib.rs, crates/nvs-cli/tests/test_dir.rs, docs/rules/testing/a-directory-of-programs-is-one-test-program.md, docs/rules/packaging/nothing-runs-before-the-program.md]
  - id: 5
    title: nvs test runs no test of a package the program loads
    status: open
    context: [crates/nvs-types/src/test_inherit.rs, crates/nvs-cli/tests/test_dir.rs, docs/rules/testing/test-attribute.md, docs/rules/testing/hook-inheritance.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: one-judge-of-a-package-path
    stage: 1
    name: "every reader of a package path refuses an empty, dot or dot-dot part, and a git tree entry that is not one name is refused before a file is written"
    run: [cargo, test, --bin, nvs, --, an_empty_dot_or_dot_dot_part_is_refused, every_reader_of_a_package_path_refuses_the_same_parts, a_tree_entry_that_is_not_one_name_is_refused]
    timeout: 1800
    tests:
      - an_empty_dot_or_dot_dot_part_is_refused
      - every_reader_of_a_package_path_refuses_the_same_parts
      - a_tree_entry_that_is_not_one_name_is_refused
  - id: one-manifest-by-name
    stage: 2
    name: "the one check of a carried file name passes the root package.toml alone, and a download path is held to it"
    run: [cargo, test, --bin, nvs, --, only_the_root_package_toml_is_carried, a_download_is_never_a_configuration_file]
    timeout: 1800
    tests:
      - only_the_root_package_toml_is_carried
      - a_download_is_never_a_configuration_file
  - id: one-manifest-at-pack
    stage: 2
    name: "nvs pkg pack refuses a listed second package.toml"
    run: [cargo, test, --test, package_client, --, a_listed_second_package_toml_is_refused]
    timeout: 1800
    tests:
      - a_listed_second_package_toml_is_refused
  - id: one-manifest-at-fetch
    stage: 2
    name: "a fetched archive that carries a second package.toml is refused and leaves nothing behind"
    run: [cargo, test, --test, package_fetch, --, a_fetched_archive_that_carries_a_second_package_toml_is_refused_and_writes_nothing]
    timeout: 1800
    tests:
      - a_fetched_archive_that_carries_a_second_package_toml_is_refused_and_writes_nothing
  - id: one-manifest-at-the-registry-build
    stage: 2
    name: "nvs registry build refuses a commit that carries a second package.toml"
    run: [cargo, test, --test, registry_build, --, a_commit_that_carries_a_second_package_toml_is_refused]
    timeout: 1800
    tests:
      - a_commit_that_carries_a_second_package_toml_is_refused
  - id: outermost-package-decides
    stage: 3
    name: "a second manifest inside a package's folder, named or not, leaves every file below it under the package's namespace check"
    run: [cargo, test, --test, package_grants, --, a_second_manifest_with_no_name_does_not_end_the_namespace_check, a_second_manifest_with_a_name_makes_no_second_package]
    timeout: 1800
    tests:
      - a_second_manifest_with_no_name_does_not_end_the_namespace_check
      - a_second_manifest_with_a_name_makes_no_second_package
  - id: registry-compares-exactly
    stage: 3
    name: "nvs registry build refuses a declaration whose namespace differs from the package's name only in case"
    run: [cargo, test, --test, registry_build, --, a_namespace_that_differs_only_in_case_is_refused]
    timeout: 1800
    tests:
      - a_namespace_that_differs_only_in_case_is_refused
  - id: no-walk-into-a-package
    stage: 4
    name: "nvs test and nvs check over a folder load no file of a package below it, and a package's folder named on the command line is run"
    run: [cargo, test, --test, test_dir, --, a_package_below_the_folder_is_not_walked, a_folder_check_reads_no_file_of_a_package_below_it, a_package_folder_named_on_the_command_line_is_run]
    timeout: 1800
    tests:
      - a_package_below_the_folder_is_not_walked
      - a_folder_check_reads_no_file_of_a_package_below_it
      - a_package_folder_named_on_the_command_line_is_run
  - id: no-case-walk-into-a-package
    stage: 4
    name: "the case walk skips a folder below the named one that holds a package.toml, and walks it when it is named"
    run: [cargo, test, --lib, --, a_folder_with_a_package_toml_is_not_walked_for_cases]
    timeout: 1800
    tests:
      - a_folder_with_a_package_toml_is_not_walked_for_cases
  - id: walk-and-compiler-name-one-file
    stage: 4
    name: "the walk and the compiler name the same manifest file"
    run: [cargo, test, --bin, nvs, --, the_walk_and_the_compiler_name_the_same_manifest]
    timeout: 1800
    tests:
      - the_walk_and_the_compiler_name_the_same_manifest
  - id: no-test-of-a-loaded-package
    stage: 5
    name: "nvs test runs no test of a package class the program loads, and a package named on the command line runs its own"
    run: [cargo, test, --test, test_dir, --, a_package_class_the_program_loads_runs_no_test, a_package_named_on_the_command_line_runs_its_own_tests]
    timeout: 1800
    tests:
      - a_package_class_the_program_loads_runs_no_test
      - a_package_named_on_the_command_line_runs_its_own_tests
---

# a fetched package stays in its folder and in its namespace

## Why

A package's files reach a machine three ways: `nvs pkg fetch` unpacks a registry archive, a git
dependency is cloned and packed, and `nvs registry build` clones a publisher's repository. Two promises
hold everything after that. Every file lands inside the package's folder. Every file declares the
package's namespace, so the `[grants]` line `nvs pkg fetch` wrote is the line that judges it. Five
places do not keep them.

- **A git tree entry is written where its name says.** `Walk::entries` takes each name as gix decodes
  it (`crates/nvs-cli/src/package/git.rs:697`), and gix returns every byte up to the NUL.
  `Walk::place` joins the name onto the clone folder (`crates/nvs-cli/src/package/git.rs:724`,
  `crates/nvs-cli/src/package/git.rs:759`). The one check on the text is `pack::check_path`
  (`crates/nvs-cli/src/package/pack.rs:450`), whose loop over the parts refuses nothing for an empty
  part (`crates/nvs-cli/src/package/pack.rs:463`). A name that starts with `/` passes, and
  `Path::join` with a rooted path drops the folder. `nvs pkg fetch` of a git dependency
  (`crates/nvs-cli/src/package/resolve.rs:433`) and `nvs registry build`
  (`crates/nvs-cli/src/package/build.rs:169`) both write the file at that absolute path. The fetch
  prints its normal lines and exits 0.
- **Three readers each carry their own copy of the missing check.** `listed_form`
  (`crates/nvs-cli/src/package/pack.rs:427`), `unpack_into` (`crates/nvs-cli/src/package/fetch.rs:537`)
  and `download::check_path` (`crates/nvs-cli/src/package/download.rs:118`) each refuse an empty part
  by hand. The git walk is the reader that has no copy.
- **A second `package.toml` ends the namespace check.** `Finder::package_of` walks up from a loaded
  file's folder and stops at the first folder that holds a `package.toml`
  (`crates/nvs-types/src/package.rs:174`). A manifest with no `[package] name` leaves the file in no
  package (`crates/nvs-types/src/package.rs:202`). A manifest with another name makes the files below
  it a second package. Either way the file's namespace matches no `[grants]` line, and a namespace no
  line matches holds the application's authority (`crates/nvs-types/src/capability.rs:247`). With
  `'Acme\Http\*' = []` written, a file under `vendor/acme/http/src/Inner/` beside a one-line
  `package.toml` declares `namespace Acme\HTTP;` and reads the application's file. With that manifest
  removed, the same program is `E0660`.
- **Nothing refuses the second manifest.** `check_not_server_config` refuses a carried `nvs.toml`
  alone (`crates/nvs-cli/src/package/pack.rs:495`), for `pack`, the fetch's unpack and the registry
  build. `download::check_path` does not call it, so a `[[download]]` entry with `path = "nvs.toml"`
  passes. The registry build's own check, `inside`, folds case
  (`crates/nvs-cli/src/package/build.rs:385`), and the grant lookup is exact.
- **`nvs test <dir>` and `nvs check <dir>` load every file under `vendor/`.** `programs_under` skips a
  linked folder and a cache folder and nothing else (`crates/nvs-cli/src/main.rs:3366`), and
  `nvs_test::collect` does the same for `.nvst` cases (`crates/nvs-test/src/lib.rs:237`). The entry of
  a folder run requires every file found, so every source file of every fetched package is loaded by
  `require`, and `nvs test` runs each `#[Test]` method a dependency declares.
  `rule:packaging/nothing-runs-before-the-program` says a dependency's first line runs when the
  program calls it. A run over one file does the same. `check_class_tests` records the `#[Test]`
  methods of every class the compile checks (`crates/nvs-types/src/testing.rs:574`). So
  `nvs test AppTest.nvs` runs the tests of each package class it loads by `require` or `autoload`.

Each is reached by the author of a dependency, direct or transitive, or by anybody who may publish to
a registry. The first writes a file on the developer's machine, the CI runner or the registry host.
The second and third give a package with an empty grant line everything the application holds.

## Standing decisions

**`pack::check_path` is the one judge of a package path.** It refuses an empty part, `.` and `..` by
name, before its other part rules. So an empty path, a leading `/`, `//` and a trailing `/` are refused
for the archive, the git walk, the `files` list and a download alike. The private copies are deleted:
`listed_form`, the `split('/').any(str::is_empty)` test in `unpack_into`, and the first block of
`download::check_path`. A `\` is already refused by `check_path`'s character rule. `folder_path`
(`crates/nvs-cli/src/package/git.rs:186`) stays: it judges `--dir`, which is a folder of a repository
and not a path the archive carries. **What it spends:** nothing. One compare more per part, at fetch
and pack time, and three blocks fewer. Usability: the `files` list and a download lose their own
sentence and print the shared one. Safe default: if one caller needs words of its own, it adds them
around the shared refusal and never checks the parts again. Declined: a fourth copy in the git walk.
That is the shape that left the walk open.

**A git tree entry is one path component.** `Walk::entries` refuses a name that is empty or holds `/`,
for every entry of every tree it reads, also one the archive would not carry. A repository `git` made
never holds such a name. **What it spends:** one scan of each name. Declined: a second check of the
resolved path after the join. The clone folder is new, a link is refused, and one text check that
every caller passes is the root fix. A second check would be an invariant each new writer has to
remember.

**A package carries one `package.toml`, its own.** `check_not_server_config` is renamed
`check_reserved_file`, for what it now does. It refuses an `nvs.toml` in any folder and any case of
letters, as now. It also refuses every path whose last part is `package.toml` in any case of letters,
unless the whole path is exactly `package.toml`. `pack`, the unpack and the registry build already
call it, and `download::check_path` calls it too, which closes the `[[download]]` sibling in the same
edit. A download named exactly `package.toml` is refused already, as a file the archive contains
(`crates/nvs-cli/src/package/pack.rs:258`). **What it spends:** one compare per carried path.
Usability: a package that kept a second manifest as data renames the file. Declined: allowing a second
manifest that has no `[package]` table. The check would have to read the file, and the compiler would
have to agree with it on every tree.

**The compiler does not trust that a fetch wrote the tree.** The compiler also runs on a tree a person
copied or edited, so `Finder::package_of` closes the hole on its own.

- What the code does today. `stop` is the canonical folder of the entry file
  (`crates/nvs-types/src/package.rs:82`). The walk goes up from a loaded file's folder and ends at the
  first ancestor that is `stop` or above it (`crates/nvs-types/src/package.rs:188`). So a
  `package.toml` in the entry's folder or above it is the application's, and is never read as a
  package. A package developed in its own folder is therefore an application while its entry is
  inside it. The walk also ends at the first `package.toml` it finds, whatever that file holds.
- What changes. A manifest with no `[package] name` does not end the walk. A named manifest does not
  end it either: the walk goes on to `stop`, and the outermost named manifest below `stop` is the
  file's package. Its namespace check and its path check then cover every file under its folder. An
  inner manifest is never read. A file with no named manifest between it and `stop` is in no package,
  as now, so `code_with_no_package_name_gets_no_file_check` stays green.
- What the tree holds. 49 `package.toml` files are tracked. Each is an application's manifest beside
  its entry files, a package at `vendor/<vendor>/<name>/` below one, `packages/rest/package.toml`, or
  one of the two fixtures under `crates/nvs-cli/tests/fixtures/package-api/versioned/`. None lies
  inside another package's folder. The tests that build a layout write a root `package.toml` or
  `vendor/acme/http/package.toml` (`crates/nvs-cli/tests/package_grants.rs:57`). No tracked layout
  changes.

**What it spends:** one `stat` per folder between a package's folder and the entry's, once per
compile, where the walk stopped at the package before. Nothing per request. **What it breaks:** a
hand-vendored package kept inside another package's folder is judged as part of the outer package.
It moves beside it. A package that vendors its own dependencies under its own `vendor/` is the same
case when an application loads it. Safe default: where the session finds a layout in the tree that
nests, the inner package moves beside the outer one and the check stays. Declined: the nearest
manifest wins, as now. It lets a package hand part of its own tree to another name.

**`rule:security/an-unmatched-namespace-holds-the-application` stays as it is.** This goal fixes the
two checks that keep a package's files inside the package's namespace. It does not change what an
unmatched namespace holds. The rule's Declined list has one entry, deny by default, and says nothing
about a file that lies inside a package's folder. `### Backlog` holds that line, and no stage edits the
rule.

**The registry build compares a namespace exactly**, as `Package::owns` does
(`crates/nvs-config/src/package.rs:116`). `inside` drops both `to_lowercase` calls. **What it
spends:** nothing. It breaks a package whose classes are written in another case than its name, which
the client's `E0660` already refuses.

**`nvs test <dir>` and `nvs check <dir>` do not walk into a folder below `<dir>` that holds a
`package.toml`**, named or not. The folder named on the command line is always walked, so a package's
tests run when somebody names the package's folder. The check is `is_file` on `package.toml`, one
`stat` per folder walked, and it reads no manifest, so a manifest with no name cannot opt a folder
back in. `programs_under` and `nvs_test::collect` call one function, `nvs_test::is_walked`, which also
holds the cache test both walks make today. `nvs-test` has no dependencies, so the file name is a
constant there, and a test in `nvs-cli` holds it equal to `nvs_config::grants::FILE`. **What it
spends:** one `stat` per folder walked, before the compile. Usability: a project that keeps a second
project with its own `package.toml` in a subfolder runs that one by naming it, and `nvs test --list`
lists by the same walk, so the editor's test list follows. Safe default: skip. Declined: a dependency
of `nvs-test` on `nvs-config` for one file name. Declined: skipping a folder named `vendor`. A name is
a convention, and a hand-copied package sits anywhere.

Who relies on the walk today, as read before this goal was written:

- `tools/nv/cmd/site.ts:2362` runs `nvs test .` in each folder under `apps/`. No app holds a
  `package.toml`.
- `tools/nv/cmd/deep.ts:183`, `tools/nv/lib/chain.ts:137` and `tools/nv/select/record.ts:456` run
  `.nvst` trees. No `.nvst` file lies under a folder that holds a `package.toml`.
- `crates/nvs-cli/tests/test_dir.rs:37` builds fixtures with no `package.toml`.
- `crates/nvs-cli/src/package/api.rs:175` walks a package's `src/`. Stage 2 refuses a `package.toml`
  below the root before that walk runs (`crates/nvs-cli/src/package/build.rs:177`).
- The proofs runner starts every example and attack by file (`tools/nv/proofs/run.ts:806`).

No caller relies on walking into a nested manifest, so stage 4 has no item for one.

**A `#[Test]` method is collected only from a class whose file belongs to the project under test.**
The test table keeps no row for a class whose file lies in a package below the entry's folder.
`Packages::of` answers that for a file (`crates/nvs-types/src/package.rs:54`). It finds no package for
a file when the entry lies in that file's package, because a `package.toml` in the entry's folder or
above it is the application's (`crates/nvs-types/src/package.rs:188`). So a package's tests run when
its folder, or a file in it, is the entry. A folder run has an entry too, in the folder that was named
(`crates/nvs-cli/src/main.rs:3331`). This is the one place every way of loading a class passes:
`require`, `autoload` and the folder walk. The folder walk rule stays, because it also keeps
`nvs check <dir>` from loading a package's files.

- Where the rows are removed. `test_inherit::merge` runs once every class is checked
  (`crates/nvs-types/src/check.rs:532`). It already removes the rows of each abstract class, after
  each subclass has taken its copies (`crates/nvs-types/src/test_inherit.rs:276`). The rows of a
  package's class are removed in the same step.
- What an application's subclass keeps. A class of the application that extends a package's test class
  runs the tests it inherits, under its own name. `rule:testing/hook-inheritance` says the same of an
  abstract base class.
- What reaches `merge`. It takes the class graph and the table today
  (`crates/nvs-types/src/test_inherit.rs:247`). It gains the symbol table and the `Packages`.
  A class's file is its symbol's `decl_span.file` (`crates/nvs-hir/src/symbol.rs:87`,
  `crates/nvs-hir/src/symbol.rs:132`). `check_files` holds the `Packages` and passes it through
  `after_walk` (`crates/nvs-types/src/check.rs:266`, `crates/nvs-types/src/check.rs:380`).
- What follows the rows. `forget_tests` removes a class's `#[Test]` and `#[Property]` rows
  (`crates/nvs-types/src/expr_table.rs:2651`), and a `#[Bench]` is a `#[Test]` row
  (`crates/nvs-types/src/testing.rs:3`). So a package's properties and benches leave the table with its
  tests. Its fixtures and hooks stay, for a subclass to read. `nvs test --list` reads the same table
  (`crates/nvs-cli/src/runner.rs:4653`).
- Who passes no `Packages`. The editor does (`crates/nvs-lsp/src/document.rs:1149`), and so does every
  caller of `check_program`. For each of them the table is as it is today.
- Who relies on a package's rows today, as read before this goal was written. No tracked `.nvs` file
  under a `vendor/` folder has a `#[Test]`, `#[Bench]` or `#[Property]` method. The one tracked package
  keeps its tests in its own folder, and every entry there lies in the package
  (`packages/rest/tests/bootstrap.nvs:1`).

**What it spends:** one lookup per class that has a `#[Test]` or `#[Property]` row, once per compile.
Nothing at run time. Usability: an application that ran a dependency's tests by loading its classes
runs them from the package's folder. Safe default: a class whose file cannot be placed keeps its rows.
Declined: an option to run a dependency's tests from the application. A package's tests run from the
package's own folder. Declined: leaving the rows out in `check_class_tests`
(`crates/nvs-types/src/testing.rs:374`). A subclass in the application would lose the tests it
inherits.

**A refusal names the file and says what to do**, as the refusals in `pack.rs` do. The sentences are
settled here, and each follows `AGENTS.md` § *Text an end user reads*:

- `check_path`: "the path `{path}` has an empty part, or a part that is `.` or `..`. Write the path of
  one file inside the package, with `/` between folders, such as `templates/error.html`."
- `Walk::entries`: "`{url}` has an entry named {name:?}. A name in a git folder is one file or folder
  name, so it cannot be empty or contain `/`."
- `check_reserved_file`: "`{path}` is a second `package.toml`. A package has one `package.toml`, in
  its top folder. Delete the file or give it another name."

**Each fix gets the attack the hostile runner can deliver, and a Rust test holds the rest.** The
runner starts one `.nvs` file with `nvs run` or `nvs test` (`tools/nv/proofs/run.ts:806`). It cannot
start `nvs pkg fetch`, hand it a commit, or run `nvs test` over a folder.

- The namespace fix gets two attacks the compiler refuses, one file per refusal, because a file marked
  `// hostile: expect-refusal` passes on its first.
- The path fix gets one attack written as
  `tests/hostile/tools/packages/nvs-pkg-publish/01-a-token-outside-its-scope.nvs` is: the program holds
  the path rules in a class, and each step gives it one name. The Rust tests hold the tool.
- The walk gets no hostile case. A case run by file never loads the package's test file, so it would
  report that an attack was tried when it was not (`tests/hostile/README.md` § *When a case actually
  breaks something*). Its attack is the layout `a_package_below_the_folder_is_not_walked` builds.
- The test table gets no hostile case. A case marked `// proof: test` passes when the report reaches
  its summary line, with the package's test in the report or not. So the case would pass with the fix
  taken out. Its attack is the layout `a_package_class_the_program_loads_runs_no_test` reads.

Attacks go in `tests/hostile/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/`. The folder
`grants-what-each-namespace-may-do/` is left alone: goal `package-authority-closed` writes its files
`03` to `09` there.

**Lanes:** one lane, in order. Stages 1 and 2 share `crates/nvs-cli/src/package/pack.rs`, `fetch.rs`
and `download.rs`. Stages 2 and 3 share `crates/nvs-cli/tests/registry_build.rs`. Stage 4 touches
files no earlier stage does, and runs after them because its note on `api.rs` rests on stage 2.
Stages 4 and 5 share `crates/nvs-cli/tests/test_dir.rs`, `crates/nvs-cli/src/runner.rs`,
`website/src/content/docs/tools/test-runner/index.mdx` and
`docs/rules/packaging/nothing-runs-before-the-program.md`. Stage 5 reads `Packages::of` and edits
nothing in `crates/nvs-types/src/package.rs`, which stage 3 rewrites. Goal
`package-authority-closed` runs later on the chain and holds anchors into
`crates/nvs-cli/tests/package_grants.rs`, so stage 3 adds its tests after the last test of that file.

## Stages

### 1. one function judges every package path

**Does:** make `pack::check_path` refuse an empty, `.` and `..` part, delete the three private copies
of that check, and refuse a git tree entry that is not one name.

- [x] `check_path` refuses an empty part, `.` and `..` by name, before the reserved-name rule, with
  the sentence the standing decision gives. Its doc lists the new reason
  (`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`,
  `crates/nvs-cli/src/package/pack.rs:450`, `crates/nvs-cli/src/package/pack.rs:463`).
- [x] `listed_form` and its call are deleted, and `file_set`'s doc follows
  (`crates/nvs-cli/src/package/pack.rs:427`, `crates/nvs-cli/src/package/pack.rs:227`,
  `crates/nvs-cli/src/package/pack.rs:195`). Test `a_files_line_must_be_one_relative_path` is deleted
  with it, and its lines move into the new test below (`crates/nvs-cli/src/package/pack.rs:722`).
- [x] Test `an_empty_dot_or_dot_dot_part_is_refused` in `pack.rs`'s tests
  (`crates/nvs-cli/src/package/pack.rs:690`). It gives `check_path` the empty path, `/abs.txt`,
  `//host/share.txt`, `a//b.txt`, `src/`, `.`, `..`, `./a.txt`, `../a.txt`, `a/./b.txt` and
  `a/../b.txt`, and expects the new sentence for each.
- [x] `unpack_into` loses its own empty-part test, and the docs of `unpack` and of the module follow
  (`crates/nvs-cli/src/package/fetch.rs:537`, `crates/nvs-cli/src/package/fetch.rs:499`,
  `crates/nvs-cli/src/package/fetch.rs:14`).
- [x] `download::check_path` loses its first block and keeps the `.nvs` rule and the `src/` rule
  (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`,
  `crates/nvs-cli/src/package/download.rs:117`). The test at
  `crates/nvs-cli/src/package/download.rs:676` follows the new sentence.
- [x] Test `every_reader_of_a_package_path_refuses_the_same_parts` in `pack.rs`'s tests. One table of
  bad paths goes to each reader: `file_set` through a manifest's `files`, `fetch::unpack` through an
  archive, and `download::check_form`. It fails naming the reader and the path that passed. The
  archive's entry names are written into the header's name field as bytes, because `set_path` refuses
  some of them (`crates/nvs-cli/src/package/pack.rs:201`, `crates/nvs-cli/src/package/fetch.rs:502`,
  `crates/nvs-cli/src/package/download.rs:71`).
- [x] `Walk::entries` refuses a name that is empty or holds `/`, with the sentence the standing
  decision gives (`rule:packaging/a-git-dependency-is-root-only`,
  `crates/nvs-cli/src/package/git.rs:697`).
- [x] Test `a_tree_entry_that_is_not_one_name_is_refused` in `git.rs`'s tests
  (`crates/nvs-cli/src/package/git.rs:817`). The test builds its repository itself, with no `git`
  program: `repo.write_blob` for each file, `repo.write_object(gix::objs::Tree { entries })` for the
  root tree, and `new_commit_as` as `repository` does. `write_object` refuses only a NUL in a name, and
  asserts in a debug build that the entries are sorted, so the test sorts them. The root tree holds
  `LICENSE`, `package.toml` and one more blob entry. The test runs once per name of that entry: `/`
  followed by the scratch folder's own path with `/` between its parts and no drive letter, then
  `/landing/escaped.txt`; `src/../../escaped.txt`; and the empty name. Each `clone` over the `file:`
  remote is an error whose text names the entry, and no `escaped.txt` exists under the scratch folder
  or at the rooted path afterwards. A name gix's own decoder refuses is left out of the list.
  `nvs registry build` reaches the same walk through `clone_within`
  (`crates/nvs-cli/src/package/git.rs:136`, `crates/nvs-cli/src/package/build.rs:169`).
- [x] Attack `tests/hostile/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/02-a-file-name-that-leaves-the-folder.nvs:1`.
  A class holds the rules for one path of a package: no empty part, no `.`, no `..`, no `\` and no
  `:`. Each numbered step gives it one name and catches the error, so the next step runs: a name that
  starts with `/`, one that starts with `//`, one with `..` in the middle, one that ends in `/`, and
  the empty name. The program runs to its last line. Its comments follow `AGENTS.md` § *Text an end
  user reads*.
- [x] Rule fragment: the bullet *Every path works on every platform* says no part is empty, `.` or
  `..`, and the closing paragraphs say `pack::check_path` is the one check the pack, the fetch, a
  download and the git walk share, and that a git tree entry is one name
  (`docs/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.md:16`,
  `docs/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.md:78`).
  `rule:packaging/a-git-dependency-is-root-only` does not speak of paths and stays as it is.
  `bun nv rules --render` writes the chapter.

### 2. a package carries one package.toml, its own

**Does:** rename the one check of a carried file name, make it refuse every `package.toml` but the
root one, and call it for a download path.

- [x] `check_not_server_config` becomes `check_reserved_file`. It keeps the `nvs.toml` refusal and
  refuses a last part equal to `package.toml` in any case of letters unless the path is exactly
  `package.toml`, with the sentence the standing decision gives. Its doc and the module doc are
  rewritten as a whole (`rule:packaging/a-package-cannot-reach-its-host`,
  `crates/nvs-cli/src/package/pack.rs:495`, `crates/nvs-cli/src/package/pack.rs:60`,
  `crates/nvs-cli/src/package/pack.rs:8`).
- [x] The two callers take the new name, and their docs follow
  (`crates/nvs-cli/src/package/pack.rs:512`, `crates/nvs-cli/src/package/fetch.rs:541`,
  `crates/nvs-cli/src/package/fetch.rs:499`).
- [x] `download::check_path` calls `check_reserved_file` after `pack::check_path`
  (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`,
  `crates/nvs-cli/src/package/download.rs:130`).
- [x] Tests `only_the_root_package_toml_is_carried` in `pack.rs`'s tests and
  `a_download_is_never_a_configuration_file` in `download.rs`'s tests
  (`crates/nvs-cli/src/package/pack.rs:690`, `crates/nvs-cli/src/package/download.rs:676`). The first
  passes `package.toml` and refuses `src/Inner/package.toml`, `Package.toml`, `data/PACKAGE.TOML`,
  `nvs.toml` and `config/Nvs.Toml`. The second refuses a `[[download]]` whose `path` is `nvs.toml` and
  one whose `path` is `data/package.toml`.
- [x] Test `a_listed_second_package_toml_is_refused`, beside `a_listed_nvs_toml_is_refused`: `files`
  lists `src/Inner/package.toml`, and `nvs pkg pack` prints the new sentence
  (`crates/nvs-cli/tests/package_client.rs:297`).
- [x] Test `a_fetched_archive_that_carries_a_second_package_toml_is_refused_and_writes_nothing`.
  `assert_refused` takes the sentence it expects, and the test runs it for `src/Inner/package.toml`,
  `data/Package.TOML` and `PACKAGE.toml`, with the `// covers:` line the tests beside it carry
  (`crates/nvs-cli/tests/package_fetch.rs:427`, `crates/nvs-cli/tests/package_fetch.rs:460`).
- [x] Test `a_commit_that_carries_a_second_package_toml_is_refused`, beside
  `a_class_outside_the_package_namespace_is_refused`: the commit's `files` lists
  `src/Inner/package.toml`, and the build is refused with the new sentence
  (`crates/nvs-cli/tests/registry_build.rs:931`).
- [x] Rule fragments: a package may not contain a second `package.toml`, why, and the new function
  name (`docs/rules/packaging/a-package-cannot-reach-its-host.md:3`,
  `docs/rules/packaging/a-package-cannot-reach-its-host.md:17`). The `files` bullet says a
  `package.toml` below the root is refused
  (`docs/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.md:7`).
  `bun nv rules --render` writes the chapter.
- [x] `about.md` of the two features gains one sentence each, written by `AGENTS.md` § *Text an end
  user reads*: a package has one `package.toml`
  (`docs/examples/tools/packages/nvs-pkg-pack/about.md:10`), and a package that contains a second
  `package.toml` stops the fetch
  (`docs/examples/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/about.md:19`).

### 3. the outermost package decides a file's namespace, and the registry build compares exactly

**Does:** make `Finder::package_of` walk to the entry's folder and take the outermost named manifest,
and make the registry build's namespace check exact.

**Split:** an `Explore` subagent lists every `package.toml`, tracked or written by a test, that lies
inside the folder of a package: a folder whose own `package.toml` has a `[package] name`. The answer is
`path:NN` and one line each. The tracked tree holds none, as the standing decision says.

- [ ] `Finder::package_of` does not end at a manifest. It notes each folder that holds a
  `package.toml` on the way to `stop`, then reads them from the outermost inward and takes the first
  that names a package. A manifest with no name is passed over, and one below the package found is
  never read. `dirs` keeps the answer of every folder walked
  (`rule:packaging/a-package-file-stays-under-its-declared-prefix`,
  `crates/nvs-types/src/package.rs:174`, `crates/nvs-types/src/package.rs:163`).
- [ ] The autoload-site lookup takes the one package whose folder holds the site's base, since no
  package's folder is inside another's (`rule:packaging/a-package-cannot-reach-its-host`,
  `crates/nvs-types/src/package.rs:129`).
- [ ] The module doc is rewritten as a whole: the outermost named manifest below the entry's folder
  wins, a manifest with no name ends nothing, and the cost line names the folders between a package
  and the entry (`crates/nvs-types/src/package.rs:5`, `crates/nvs-types/src/package.rs:28`).
- [ ] Tests `a_second_manifest_with_no_name_does_not_end_the_namespace_check` and
  `a_second_manifest_with_a_name_makes_no_second_package`, after the last test of the file
  (`crates/nvs-cli/tests/package_grants.rs:201`). Each layout has `main.nvs`, which requires
  `vendor/acme/http/src/Inner/Run.nvs`, and `vendor/acme/http/package.toml` naming `Acme\Http`. In the
  first, `src/Inner/package.toml` holds one comment and `Run.nvs` declares `namespace Acme\HTTP;`. In
  the second, it names `Free\Hands` and `Run.nvs` declares `namespace Free\Hands;`. Both are `E0660`
  naming the package `Acme\Http`. Each has its control: the same layout with `Run.nvs` in
  `namespace Acme\Http\Inner;` compiles. `code_with_no_package_name_gets_no_file_check` stays as it is
  (`crates/nvs-cli/tests/package_grants.rs:117`).
- [ ] Every layout the subagent named moves its inner package beside the outer one.
- [ ] `inside` compares exactly: both `to_lowercase` calls go, and its doc says so
  (`rule:classes/names-resolve-case-sensitively`, `crates/nvs-cli/src/package/build.rs:385`,
  `crates/nvs-cli/src/package/build.rs:380`).
- [ ] Test `a_namespace_that_differs_only_in_case_is_refused`, beside
  `a_class_outside_the_package_namespace_is_refused`: `src/Other.nvs` declares `namespace Acme\HTTP;`
  in the package `Acme\Http` (`crates/nvs-cli/tests/registry_build.rs:931`).
- [ ] Attacks, each marked `// hostile: expect-refusal`, each with its files under the folder's
  `vendor/acme/http/`:
  `tests/hostile/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/03-a-second-package-toml-with-no-name.nvs:1`
  requires `vendor/acme/http/src/Inner/Run.nvs`, which declares `namespace Acme\HTTP;` beside a
  `package.toml` that holds one comment, and reads a file of the application.
  `tests/hostile/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/04-a-second-package-toml-with-another-name.nvs:1`
  requires `vendor/acme/http/src/Loose/Run.nvs`, which declares `namespace Free\Hands;` beside a
  `package.toml` that names `Free\Hands`. Every new `.nvs` file passes `nvs fmt --check`.
- [ ] Rule fragment: the last paragraph says the outermost named manifest between the file and the
  entry's folder decides, a manifest with no name ends nothing, and a package inside another package's
  folder moves beside it. The second paragraph no longer says a file belongs to a package by its
  autoload root
  (`docs/rules/packaging/a-package-file-stays-under-its-declared-prefix.md:26`,
  `docs/rules/packaging/a-package-file-stays-under-its-declared-prefix.md:5`). A **Declined:** entry
  says why the nearest manifest does not win. `bun nv rules --render` writes the chapter.

### 4. nvs test and nvs check walk into no package below the folder they were given

**Does:** make the two folder walks skip a folder below the named one that holds a `package.toml`,
through one shared function, and say so in the help of both commands.

- [ ] `nvs_test::is_walked(dir)` beside `is_cache_dir`: false for a cache folder and for a folder
  where `package.toml` is a file. The file name is the constant `nvs_test::PACKAGE_FILE`, and the doc
  says what the check spends and that it reads no manifest
  (`rule:testing/a-directory-of-programs-is-one-test-program`, `crates/nvs-test/src/lib.rs:274`).
- [ ] `collect` and `programs_under` call it where they call `is_cache_dir` today, and the comment
  above each is rewritten (`crates/nvs-test/src/lib.rs:254`, `crates/nvs-cli/src/main.rs:3388`,
  `crates/nvs-cli/src/main.rs:3364`).
- [ ] Test `a_folder_with_a_package_toml_is_not_walked_for_cases` in `nvs-test`'s lib tests: a tree
  with `first.nvst`, `vendor/acme/http/package.toml` and `vendor/acme/http/tests/second.nvst`.
  `discover` over the tree returns the first case alone, and `discover` over `vendor/acme/http`
  returns the second (`crates/nvs-test/src/lib.rs:573`).
- [ ] Test `the_walk_and_the_compiler_name_the_same_manifest` in `nvs-cli`: `nvs_test::PACKAGE_FILE`
  equals `nvs_config::grants::FILE` (`crates/nvs-cli/src/runner.rs:5585`).
- [ ] Tests in `test_dir.rs`, on one layout `fixture` builds: `AppTest.nvs` with one passing test,
  `vendor/acme/http/package.toml` naming `Acme\Http`, `vendor/acme/http/src/ClientTest.nvs` with one
  failing test, `vendor/acme/http/src/Broken.nvs` with a type error, and `vendor/acme/tools/` with a
  `package.toml` that holds one comment and a second failing test
  (`crates/nvs-cli/tests/test_dir.rs:37`, `crates/nvs-cli/tests/test_dir.rs:120`).
  `a_package_below_the_folder_is_not_walked`: `nvs test <root>` exits 0 and its report names
  `AppTest` alone. `a_folder_check_reads_no_file_of_a_package_below_it`: `nvs check <root>` prints
  `no errors`. `a_package_folder_named_on_the_command_line_is_run`: `nvs test <root>/vendor/acme/tools`
  runs that folder's test.
- [ ] The help of `nvs test` gains two sentences after the one on subfolders: "A subfolder that has
  its own `package.toml` is not read. To run the tests of a package, give `nvs test` the folder of
  that package." (`crates/nvs-cli/src/runner.rs:128`). The help of `nvs check` gains the first of them
  (`crates/nvs-cli/src/check.rs:61`). Both follow `AGENTS.md` § *Text an end user reads*.
- [ ] The two website pages that describe the walk gain the same sentence
  (`website/src/content/docs/tools/test-runner/index.mdx:35`,
  `website/src/content/docs/guides/how-to-use/project-layout.mdx:46`). `bun nv render --website`
  writes the generated files once the help text has changed.
- [ ] Rule fragments: the folder rule says which folders the walk skips and why, with a **Declined:**
  entry for skipping by the name `vendor`
  (`docs/rules/testing/a-directory-of-programs-is-one-test-program.md:1`). The sentence on skipped
  folders names the third kind (`docs/rules/testing/report-formats.md:46`).
  `rule:packaging/nothing-runs-before-the-program` says a folder run loads no file of a package below
  the folder unless the program loads it
  (`docs/rules/packaging/nothing-runs-before-the-program.md:21`). `bun nv rules --render` writes the
  chapters.

### 5. nvs test runs no test of a package the program loads

**Does:** remove the test rows of every class whose file lies in a package below the entry's folder,
once every class is checked, and say so in the help of `nvs test`.

- [ ] `merge` takes the symbol table and the `Packages` beside the class graph. After the pass over
  the abstract classes, it calls `forget_tests` for each class of `test_classes` whose file
  `Packages::of` places in a package. A class with no symbol, and every class where no `Packages` was
  passed, keeps its rows. Its doc says what the step spends
  (`rule:testing/test-attribute`, `crates/nvs-types/src/test_inherit.rs:247`,
  `crates/nvs-types/src/test_inherit.rs:276`, `crates/nvs-types/src/package.rs:54`,
  `crates/nvs-hir/src/symbol.rs:132`).
- [ ] `after_walk` takes the `Packages` from `check_files` and passes it, with `module.symbols`, to
  `merge` (`crates/nvs-types/src/check.rs:506`, `crates/nvs-types/src/check.rs:380`,
  `crates/nvs-types/src/check.rs:532`, `crates/nvs-types/src/check.rs:266`).
- [ ] The docs that describe the table are rewritten as a whole: the module doc's section on what
  happens after every file is checked, and the doc of `forget_tests`
  (`crates/nvs-types/src/test_inherit.rs:17`, `crates/nvs-types/src/expr_table.rs:2649`).
- [ ] Fixture `loads-a-package`, four files under `crates/nvs-cli/tests/fixtures/test-dir/`. Every
  `.nvs` file passes `nvs fmt --check`.
  `crates/nvs-cli/tests/fixtures/test-dir/loads-a-package/vendor/acme/http/package.toml:1` names
  `Acme\Http`.
  `crates/nvs-cli/tests/fixtures/test-dir/loads-a-package/vendor/acme/http/src/Client.nvs:1` declares
  `Acme\Http\Client` with a method `total` that returns `3` and one failing test,
  `aPackageTestThatFails`.
  `crates/nvs-cli/tests/fixtures/test-dir/loads-a-package/vendor/acme/http/src/Cases/ClientCase.nvs:1`
  declares the class `Acme\Http\Cases\ClientCase`, not `final`, with one passing test, `inherited`.
  `crates/nvs-cli/tests/fixtures/test-dir/loads-a-package/AppTest.nvs:1` declares
  `autoload 'Acme\Http\Cases' from 'vendor/acme/http/src/Cases';` and requires
  `vendor/acme/http/src/Client.nvs`. Its class `AppTest` has one test, `usesTheClient`, which calls
  `Client::total`. Its class `AppClientTest` extends `ClientCase` and has one test, `own`.
- [ ] Test `a_package_class_the_program_loads_runs_no_test`, after the last test of the file
  (`crates/nvs-cli/tests/test_dir.rs:176`). `nvs test loads-a-package/AppTest.nvs` exits 0, and its
  report names `AppClientTest::inherited`, `AppClientTest::own` and `AppTest::usesTheClient` alone.
  `--list` names the same three, and so does `nvs test loads-a-package`. Today the run by file also
  names `Acme\Http\Client::aPackageTestThatFails` and `Acme\Http\Cases\ClientCase::inherited`, and
  exits 1.
- [ ] Test `a_package_named_on_the_command_line_runs_its_own_tests`, beside it. It reads each report
  through `run_test`, because `document` expects a run that passes
  (`crates/nvs-cli/tests/test_dir.rs:50`). `nvs test loads-a-package/vendor/acme/http/src/Client.nvs`
  exits 1 and names `Acme\Http\Client::aPackageTestThatFails`.
  `nvs test loads-a-package/vendor/acme/http` exits 1 and names that test and
  `Acme\Http\Cases\ClientCase::inherited`.
- [ ] The help of `nvs test` gains one sentence, before the sentence "To run the tests of a package,
  …" that stage 4 wrote: "`nvs test` does not run the `#[Test]` methods of a package that your program
  loads." (`crates/nvs-cli/src/runner.rs:128`). The page on the test runner gains the same sentence
  at the same place (`website/src/content/docs/tools/test-runner/index.mdx:35`). Both follow
  `AGENTS.md` § *Text an end user reads*. `bun nv render --website` writes the generated files once the
  help text has changed.
- [ ] Rule fragments. `rule:testing/test-attribute` says the table has no row for a class whose file
  lies in a package below the entry's folder, why, and what the lookup spends. Its **Declined:** list
  gains the option that runs a dependency's tests from the application
  (`docs/rules/testing/test-attribute.md:5`, `docs/rules/testing/test-attribute.md:24`).
  `rule:testing/hook-inheritance` says a package's base class runs no test for itself under an
  application's entry, and a subclass in the application runs each test it inherits
  (`docs/rules/testing/hook-inheritance.md:24`). `rule:packaging/nothing-runs-before-the-program` says
  `nvs test` runs no `#[Test]` method of a package the program loads
  (`docs/rules/packaging/nothing-runs-before-the-program.md:21`). `bun nv rules --render` writes the
  chapters.

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done. `pack::check_path` is the one judge of a package path, and `Walk::entries`
refuses a git tree entry that is not one name. `pack::check_reserved_file` refuses an `nvs.toml` in
any folder and every `package.toml` but the root one, in any case of letters. `pack`, the unpack, the
registry build and `download::check_path` call it. The five acceptance tests of stage 2 passed once
by name. The rule chapter is rendered, and the rule's title now names the second `package.toml`
(`data/rules/packaging/a-package-cannot-reach-its-host.json:2`).

The sweep's findings were the same as the last session's and need no change. Nightly run
43001191130 tested `f4c1d5c7e2`, and its red test was fixed after it by `305c04f660`.
`env-secret-proofs` is defined nowhere in the tree and has no log: stale driver state from the
retired goal `defaults-that-work`.

`Finder::package_of` still ends at the first `package.toml`. `inside` in the registry build still
compares lower-cased. `programs_under` and `nvs_test::collect` walk every folder but a linked one and
a cache. The test table keeps the rows of every class the compile checks, also a class of a package.

### Next

Stage 3, over `crates/nvs-types/src/package.rs`, `crates/nvs-cli/src/package/build.rs`,
`crates/nvs-cli/tests/package_grants.rs`, `crates/nvs-cli/tests/registry_build.rs`, two attacks and
one rule fragment. It waits on no other goal. Start with the stage's `**Split:**` subagent.

- [ ] `crates/nvs-types/src/package.rs:174`: `Finder::package_of` walks to `stop` and takes the
  outermost named manifest; `dirs` (`crates/nvs-types/src/package.rs:168`) keeps every folder's answer.
- [ ] `crates/nvs-types/src/package.rs:129`: the autoload-site lookup, and the module doc
  (`crates/nvs-types/src/package.rs:5`).
- [ ] `crates/nvs-cli/tests/package_grants.rs:201`: the two tests after the last one, each with its
  control.
- [ ] `crates/nvs-cli/src/package/build.rs:385`: `inside` drops both `to_lowercase` calls
  (`crates/nvs-cli/src/package/build.rs:396`).
- [ ] `crates/nvs-cli/tests/registry_build.rs:931`: test
  `a_namespace_that_differs_only_in_case_is_refused`, after
  `a_commit_that_carries_a_second_package_toml_is_refused` (`crates/nvs-cli/tests/registry_build.rs:951`).
- [ ] `tests/hostile/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/03-a-second-package-toml-with-no-name.nvs:1`
  and `04-a-second-package-toml-with-another-name.nvs`.
- [ ] `docs/rules/packaging/a-package-file-stays-under-its-declared-prefix.md:26`: the fragment, then
  `bun nv rules --render`.

No test is expected red: stage 2 changed only the words `assert_refused` matches, and every caller
of it passes the words it expects.

### Backlog

- `nvs pkg fetch` does not run `pack::check_component` on a fetched `.nvsx`
  (`crates/nvs-cli/src/package/pack.rs:11`).
- `rule:security/an-unmatched-namespace-holds-the-application`'s Declined list does not speak about a
  file inside a package's folder whose namespace matches no line
  (`docs/rules/security/an-unmatched-namespace-holds-the-application.md:25`).
