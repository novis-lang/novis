---
id: cli
title: The nvs command
summary: every subcommand of the `nvs` binary — run, check, test, build, api, config, info, meta, ast, fmt — with its flags, its exit status and what it prints
keywords: nvs, nvs run, nvs check, nvs test, nvs build, --compile, --openapi, nvs api diff, nvs config check, nvs config dump, nvs info, nvs meta --json, nvs ast, nvs fmt, formatter, --check, --diff, --stdin, --config, --dump-ir, --dump-asm, --filter, --format, --mutate, --since, NoMutate, mutants.toml, mutation testing, --tag, --shard, --rerun-failed, --shuffle, --watch, --jobs, exit status, exit code, .nvs, shebang, cache, nvs cache gc, nvs cache clear, single-file executable, bundle, syntax check, version, licenses, nvs debug, --tail, --once, debug index, recorded requests, debug viewer, --listen, replay, compare requests, open in editor
---

# One binary

Novis is one program, `nvs`. You give it a command, then the options of that command, and usually
one file:

    nvs run hello.nvs
    nvs check src/app.nvs
    nvs test tests/
    nvs --version          # prints `nvs 0.0.1 (commit 16a26500b, 2026-09-24)`

| Command | What it does |
|---|---|
| `nvs run <file>` | check, compile and run a program |
| `nvs serve <file>` | run an application until it is stopped: its HTTP requests, its scheduled jobs and its queue workers. See [the server chapter](#tools-server) |
| `nvs reload` | make a running `nvs serve` use the code and configuration on disk now. See [the server chapter](#tools-server) |
| `nvs service <verb>` | install, start, stop and remove a program that runs as a system service. See [the server chapter](#tools-server) |
| `nvs check <path>` | check a file or a folder and report every error. It runs nothing |
| `nvs test <paths>...` | run a program's `#[Test]` methods, or a tree of `.nvst` cases |
| `nvs build --compile <file>` | write a single executable file that contains the program's source |
| `nvs build --openapi <file>` | write the program's OpenAPI 3.1 document to standard output |
| `nvs api diff <old> <new>` | list every change between two OpenAPI documents, and say which changes break a client |
| `nvs pkg <verb>` | add, download, update, pack and publish Novis packages. See [the packages chapter](#tools-packages) |
| `nvs ext <verb>` | create, build, test, check and pin an extension |
| `nvs init` | write a `nvs.toml` with every key commented out, into the working directory or to the path `--config` gives. See [Installing on a host](#tools-install) |
| `nvs config check [files]` | read every configuration file and report the errors in them |
| `nvs config dump [files]` | print every configuration key in force |
| `nvs queue migrate` | create the job queue's tables in its database |
| `nvs schema <verb>` | print what a schema value would change in a database, run the safe steps, or print the schema the database has |
| `nvs tmp clean` | delete the temporary files of a script that was killed |
| `nvs cache gc` and `nvs cache clear` | delete compiled programs from the compile cache. See [nvs cache](#nvs-cache) |
| `nvs update-check` | check whether a newer version of Novis is released |
| `nvs telemetry <verb>` | turn the anonymous usage counters on or off, and show what an upload sends |
| `nvs info` | build, host and third-party licensing information |
| `nvs meta --json` | print every `Core` class and function as JSON |
| `nvs agent <verb>` | the `Core` classes and functions for a coding agent: a primer, an index, a search and one page. `nvs agent init` writes files that tell an agent to use these commands. See [the agents chapter](#tools-agents) |
| `nvs doc <file>` | write a program's declarations as Markdown, one page per class |
| `nvs stubs` | write one declaration file for each `Core` class, enum and attribute |
| `nvs code <verb>` | find where a name is declared and used, print a file's outline, and rename or move code |
| `nvs ast [--json] <file>` | parse one file and print its syntax tree |
| `nvs fmt [paths]...` | rewrite files in the one standard layout, or with `--check` list the files that would change |
| `nvs lsp` | run the language server on standard input and output. See [the editor chapter](#tools-editor) |
| `nvs lsp-test <paths>...` | run a tree of `.lspt` cases against the language server. See [the editor chapter](#tools-editor) |
| `nvs dap` | run the debug adapter on standard input and output. See [the editor chapter](#tools-editor) |
| `nvs debug` | read the requests a server recorded into one database, and wait for more |

Every command also takes `--config <PATH>` (see `nvs run`), `--data <PATH>` and `-h`/`--help`.
`--data` sets the data folder, where Novis keeps its own files (see
[Installing on a host](#tools-install)). The short options `-i`, `-a`, `-r`, `-f` and `-v` (lower
case) do not exist. Each task has its own command, such as `nvs run` or `nvs info`.

**Not in this chapter yet:** `nvs queue`, `nvs schema`, `nvs tmp`, `nvs doc` and `nvs stubs` have
no section here. Run the command with `--help` to see its options. `nvs --help` lists every
command.

# Files, extensions and tags

- A program is a `.nvs` file. The extension is a convention, not a rule: the front end never reads
  it, `nvs run` accepts any path, and what matters is the content. `nvs test` is the one command
  that decides anything from an extension, because it has two suites to choose between.
- `<?` alone is not a tag. The file stays in HTML mode, and `<?` is copied to the output as text.
- A `#!` first line opens code mode. When the first two bytes of a file are `#!`, Novis skips that
  line and reads the rest as code, as if `<?nvs` stood there, so `#!/usr/bin/env nvs` makes a
  script you run as `./tool` on Linux and macOS. An `<?nvs` in such a file before any `?>` is
  `E0009`. `#!` anywhere else is ordinary text.

```nvs
#!/usr/bin/env nvs
echo 'This line runs as code.', "\n";
```
```output
This line runs as code.
```

# nvs run

    nvs run <file> [--config <path>]... [--dump-ir | --dump-asm]

`nvs run` checks the file the way `nvs check` does, stops with the same diagnostics if any, and
otherwise compiles and runs it with the current directory as its working directory. Before running
it reads the configuration:

- `./nvs.toml` in the working directory, if there is one.
- If there is none, `nvs.toml` in the data folder. The data folder is `.nvsdata` next to the `nvs`
  program, or the folder that `--data` names.
- If neither file exists, `nvs run` writes `nvs.toml` into the data folder and reads it. Every key
  in that file is commented out, so the program runs with an empty configuration. `--no-init`, or
  the environment variable `NOVIS_NO_INIT`, stops `nvs run` from writing the file. `nvs run` never
  writes a file into the working directory.
- `--config <path>` names a file to read *instead*. Then `nvs run` reads neither `nvs.toml`. Repeat
  the flag to read several files in order. A named file that does not exist stops the run with
  `E0605`. Relative paths inside a configuration file start at that file's own directory. The
  configuration chapter has the file format.

Two debugging flags replace running with printing: `--dump-ir` prints the lowered intermediate
representation of every function in the program, and `--dump-asm` prints the generated machine
code. Both are for reading the compiler's output, not the program's.

## Exit status

| Status | When |
|---|---|
| `0` | the program ran to the end, or called `exit`/`exit(0)`/`exit("message")` |
| `n` | the program called `exit(n)` |
| `1` | a diagnostic stopped the check, an uncaught throwable ended the program, or a resource limit was breached (`FATAL: …` on standard error) |
| `2` | the command line itself was wrong (an unknown subcommand or flag) |

`exit("message")` prints the message and exits `0`. An uncaught throwable writes one log record to
standard error: its message, its class and one entry per frame. `[log] format` picks how the record
is written — one JSON line by default, and readable text with one `#n Class::method() at file:line`
line per frame under `format = "text"`. Nothing about it reaches standard output.

```nvs exit=3
<?nvs
echo 'bye', "\n";
exit(3);
```
```output
bye
```

```nvs exit=1
<?nvs
echo 'before', "\n";
throw new LogicError('boom');
```
```output
before
```

# nvs check

    nvs check <path> [--autoload-map] [--root <folder>]

Prints `no errors` and exits `0` when the file and everything it reaches through `require` and
`autoload` type-checks; otherwise prints every diagnostic found and exits `1`. Nothing runs, so it
is the command for an editor, a pre-commit hook or CI.

The path can also be a directory. Then `nvs check` checks all the `.nvs` files under it as one
program. This is the same program `nvs test` runs for that directory. A class that one file declares
can be used in another file.

Each diagnostic has one shape:

```text
error[E0401]: expected `int`, found `string`
   --> bad.nvs:11:9
   |
11 | $o->add("seven");
   |         ^^^^^^^ this is `string`

error[E0405]: `Order` has no property named `totl`
   --> bad.nvs:12:6
   |
12 | echo $o->totl, "\n";
   |      ^^ referenced here

error: aborting due to 2 errors
To read what a code means and how to fix it, run `nvs agent show <code>`.
```

A code is `E` followed by four digits for an error and `W` for a warning; the location is
`file:line:column`; the caret marks the span; optional `= help:` and `= note:` lines follow. The
code is stable and is what the tables in this reference cite. The last line is printed once, after
all the diagnostics. `nvs agent show E0401` prints what the error `E0401` means and how to fix it.
`nvs check --json` does not print this line.

`nvs check --count` also prints one line on standard error with how much work the compiler did. For
a file that contains only `<?nvs` and `echo "Hello, World!\n";`, the line is:

```text
compile: tokens=7 nodes=2 names=0 exprs=0
```

`tokens` is the words and symbols in the source. `nodes` is the parts of the syntax tree. `names`
is the classes, interfaces, enums, type aliases and `use` imports. `exprs` is the expressions that
were given a type. The numbers are the same on every machine for the same program, so you can
compare a small program with a large one to see how compile time grows. `nvs run --count` prints
the same line before the program starts, with `ir`, the number of compiled instructions, added.

You can check one class file of a program, such as `nvs check app/User.nvs`. A class file that a
program loads with `autoload` cannot declare `autoload` itself. So when the file uses a class that no
file declares, `nvs check` looks for the program that loads this file with `autoload` or `require`. It searches the `.nvs` files in
the project folder in path order and uses the first program it finds. Then it checks the file with that
program's `autoload` lines, and the result is the same as for the whole program. The project folder is
the folder that `--root <folder>` names. Without `--root`, it is the folder of the `--config` file,
or else the folder you run the command in. If the file is in a folder below that one with its own
`nvs.toml` or `package.toml`, the search uses the nearest such folder. The search skips `vendor`, `target` and folders whose
names start with a dot. If no program loads the file, the errors are reported as usual. The first
error about an unknown class then has a note. The note names the folder that was searched.

A test file in a `tests` folder is checked the same way. It uses the `autoload` lines that
`tests/bootstrap.nvs` loads. A `require` in a comment or a string does not count. A test file
with its own `require` is a program of its own.

`--autoload-map` prints the resolved `autoload` map instead of `no errors` — every prefix, what a
`discover` glob skipped, what was shadowed, and every root that does not exist — so an autoload
declaration can be audited without running anything. A `{..}` prefix segment is shown as the
directory name it resolved to.

```nvs error
<?nvs
class Counter
{
    public int $n = 0;
}
Counter $c = new Counter();
$c->n = 'one';
```
```output
E0401
```

# nvs test

    nvs test <paths>... [--filter <text>] [--tag <tag>]... [--skip-tag <tag>]...
                        [--shard <n>/<m>] [--since <ref>] [--rerun-failed] [--list]
                        [--fail-fast] [--shuffle[=<seed>]] [--jobs <n>] [--show-output]
                        [--watch] [--format human|json|junit] [--update]
                        [--coverage-lcov <file>] [--coverage-clover <file>]
                        [--coverage-cobertura <file>]
                        [--profile callgrind|speedscope] [--profile-out <file>]
                        [--mutate[=<target>,...]] [--bench]
                        [--opt-level none|speed] [--probes]
                        [--cases <file>] [--record <dir>] [--shared-worker]

A path can end with a line number, such as `tests/CartTest.nvs:42`. Then only the test that
contains line 42 runs. A line inside a test class but outside its test methods runs every test of
the class. A line outside every test class stops the run with exit status `2`. The error names the
nearest test class in the file. A Windows path such as `C:\app\CartTest.nvs:42` keeps its drive
letter.

The command runs two kinds of test: programs with `#[Test]` methods, and `.nvst` case files. The
path decides which kind:

- **A `.nvs` path is a program**, and its `#[Test]` methods run. Each `public` `void` instance
  method with `#[Test]` is one test. Classes are reported in name order, and methods in the order
  they are written. A test that asserts nothing fails. `#[Test(skip: "why")]` skips the test and
  prints the reason. The `Core\Test` section has the assertions and the attribute's options.
- **A directory with `.nvs` files is one program.** It requires every `.nvs` file in the directory
  and its subdirectories, in name order, and runs their `#[Test]` methods. One file in the directory
  requires the application's bootstrap file. That file gives the whole directory its `autoload`
  declarations. A directory with both `.nvs` and `.nvst` files is an error.
- **Any other path is a `.nvst` case file, or a directory of `.nvst` files.** A case is one program
  with its expected output. The report is a summary, such as `3 passed, 0 failed, 0 skipped`. A
  program in a file whose name does not end in `.nvs` is read as a case, so it fails to parse.

One run does not mix the two kinds. The exit status is not zero when a test or a case failed. A
skipped one is not a failure. *Which kind of test to write*, in [testing](#lang-testing), says
when to use each.

```nvs test
<?nvs
use Core\Test;

final class MathTest
{
    #[Test]
    public function addsTwoNumbers(): void
    {
        Test::assertSame(2 + 2, 4);
    }

    #[Test(skip: 'not written yet')]
    public function dividesByZero(): void
    {
        Test::assertSame(1, 1);
    }
}
```
```output
MathTest
✓ addsTwoNumbers
- dividesByZero
skipped: not written yet
0 failed, 1 passed, 1 skipped, 0 flaky
```

- `--filter <text>` runs only the tests whose name contains the text: a `.nvst` case's path, or a
  program's `Class::method`. `--filter Class::` selects one class's `#[Test]` methods.
- `--tag <tag>` runs only the tests that have this tag. A test gets tags with
  `#[Test(tags: ['slow', 'db'])]`. You can give `--tag` more than once. A test then runs when it has
  at least one of the tags.
- `--skip-tag <tag>` does not run the tests that have this tag. You can give it more than once. A
  test with this tag does not run, even when it also has a tag that `--tag` names.
- `--shard <n>/<m>` runs one of `m` parts of the tests. `--shard 2/4` runs the second of four parts.
  Each test is in exactly one part, and it stays in that part when other tests are added.
- `--since <ref>` runs only the tests that a change since a git branch, tag or commit can reach. See
  *Running only what a change reaches* below.
- `--rerun-failed` runs only the tests that failed in the last run of this program. When the last
  run had no failed test, every selected test runs. You cannot use it together with `--since`.
- `--format human|json|junit` chooses how a program's run is reported.
  - `human` is the default. It prints one line per test as it runs, then
    `N failed, N passed, N skipped, N flaky in N ms`. A flaky test is a test that failed and then
    passed when it ran again.
  - `json` prints one JSON document on standard output at the end. `schemaVersion` is `2`. The
    document has a `summary` and one `tests[]` entry per test. An entry has `class`, `method`,
    `file`, `line`, `column`, `verdict` and `durationMs`. It also has `reason`, `failures` or
    `attempts` when they apply. With a coverage flag, `schemaVersion` is `3`. Each test then also
    has `coverage`: each file name with the sorted lines that test reached.
  - `junit` prints JUnit XML.

  In `json` and `junit`, what the tests `echo` goes to standard error. So standard output has only
  the document. `json` and `junit` are an error beside a `.nvst` tree.
- `--list` prints the tests without running any of them. Under `human`, it prints one
  `Class::method  file:line` line per test. Under `json`, it prints a `schemaVersion: 2` document.
  Its `listed` array has one `{class, method, file, line, column}` per test, and it has no summary.
  `--filter` selects the same tests as in a run. `--format junit`, `--update` and a `.nvst` tree
  are each an error beside it.
- `--fail-fast` starts no new test after the first test fails. Tests that are already running
  finish. The summary counts the tests that did not start as not run.
- `--shuffle` starts the tests in a random order and prints the seed in the summary.
  `--shuffle=<seed>` starts them in the same order as the run that printed that seed.
- `--jobs <n>` sets how many tests or `.nvst` cases run at once. The default is the number of
  hardware threads. `--jobs 1` runs one test at a time.
- `--show-output` prints each test's output as it runs. Without it, only a failed test's output is
  printed, under its failure.
- `--watch` runs the tests, then runs them again each time a source file of the program changes.
  Press Ctrl+C to stop. It needs the `human` format.
- `--coverage-lcov <file>` writes the run's coverage to the file in the lcov format.
  `--coverage-clover <file>` writes the same coverage as Clover XML, a format many CI
  services read. `--coverage-cobertura <file>` writes the same coverage as Cobertura
  XML, the format GitLab and Azure DevOps read. You can give all three in one run. Each line where
  a statement starts is listed with the number of times it ran. A line that no test reached has the
  count `0`. Each branch is listed too. A branch is a condition, such as the one in an `if`, a
  `while` or an `&&`. The file says how many times its true side ran and how many times its false
  side ran. The lcov and Clover files also list each function with the number of times it was
  called. A file is named relative to the directory you run `nvs test` in. Run it from your
  repository's root, so that your CI service finds the files. Under the `human` format, the run
  prints one more line, `N of N lines run (N%)`. The top-level statements of a file do not run
  under `nvs test`, so they have the count `0`. Naming any of these flags beside a `.nvst` tree
  or `--list` is an error.
- `--profile callgrind|speedscope` times every call and writes a profile file when the tests end.
  The file is `callgrind.out.<pid>` or `profile.<pid>.speedscope.json` in the current folder.
  `--profile-out <file>` writes it to another file.
- `--mutate` checks whether your tests notice a wrong line. `--mutate=<target>` changes only the
  code you name. See *nvs test --mutate* below.
- `--bench` runs the methods with `#[Bench]` and no test. It is an error beside `--mutate` or
  `--jobs`.
- `--opt-level none|speed` compiles the tests at this optimisation level. The default is `speed`,
  the level `nvs run` uses. A test that passes at one level and fails at the other means that Novis
  has a bug.
- `--probes` compiles the tests with the code that counts coverage, but writes no coverage. A test
  that passes without it and fails with it means that Novis has a bug.
- `--cases <file>` runs only the `.nvst` case files this file lists, one path per line.
- `--record <dir>` writes, for each `.nvst` case, the classes and files it used into this folder.
- `--shared-worker` also runs each `.nvst` case in one shared process, after its own run passed.
  A case fails when its output in the shared process is different. This means that the case read
  something an earlier case left behind.
- `--update` rewrites each failed `Core\Test::assertMatchesInline` snapshot in the source file. It
  is **the only flag with which `nvs test` writes to a source file**. It replaces the `$expected`
  string and nothing else. A passing snapshot does not change, and neither does any other line of
  the file. The tests that got a new snapshot are still reported as failed. Run the tests again to
  check that the new text is the one you meant. You can write the snapshot as `''`, and the first
  run fills it in. When one method has two snapshots with the same text, neither is written, and
  the run says so. `--update` is an error beside a `.nvst` tree, because a case's expected output
  is its `--EXPECT--` section.

A `.nvst` case is a list of sections, each starting with a `--SECTION--` header:

| Section | What it has |
|---|---|
| `--TEST--` | one line saying what the case checks |
| `--FILE--` | the program, run as `nvs run` runs it |
| `--EXPECT--` | its exact standard output |
| `--EXPECTF--` | its standard output, with `%s`, `%d` and `%f` placeholders |
| `--EXPECTF-ERROR--` | its standard error |
| `--FILE name--` | an extra file written beside it, such as `nvs.toml`; optional |
| `--RUN--` | another subcommand to run in that directory instead, such as `test` or `config dump --origin`; optional |

```text
--TEST--
a file inside the allowed folder can be read
--FILE nvs.toml--
[capabilities.fs]
read = ["data"]
--FILE data/note.txt--
note
--FILE--
<?nvs
echo Core\IO::read("data/note.txt");
--EXPECT--
note
```

## Running only what a change reaches

    nvs test <paths>... --since <ref> [--filter <text>] [--tag <tag>]... [--shard <n>/<m>]

`--since main` runs only the tests that a change since the git branch `main` can reach. `<ref>` can
be a branch, a tag or a commit. Novis runs `git diff` against it in the program's folder. A line you
added or edited is a change. So is every line of a new file that git does not track yet. A deleted
file is a change too. A renamed file counts as a deleted file and a new file.

Each run with `--since`, a coverage flag or `--mutate` records which methods each test ran. Novis
keeps this record in its data folder, one for each program. A test runs when one of these is true:

- in the recorded run, it ran a method you changed;
- in the recorded run, it ran a method you deleted, or a method in a file you deleted or renamed;
- you added or changed the test itself;
- the record does not have the test.

A run that leaves a test out, for example with `--filter`, does not remove it from the record. A
later `--since` run still selects it for the changes it has not run against.

When Novis cannot tell which tests a change affects, it runs more tests, never fewer. Then every
test runs, and the first line of the report says why:

- no run of this program was recorded yet;
- a line outside a method body changed, such as a property, a method's parameters, an attribute or
  a `use` line;
- a file in the program's folder that is not a `.nvs` file changed, such as `nvs.toml` or a file a
  test reads;
- a file outside the program's folder changed, and it is not one of the program's files;
- Novis cannot find a changed path, such as a deleted file whose folder is also gone.

The tests run with coverage on, so that the run can record what they reach. Use `--since` while you
edit, and run every test in CI. `--since` works with `--filter`, `--tag`, `--skip-tag` and
`--shard`: a test must match all of them. It is an error beside `--rerun-failed` or a `.nvst` tree.
The program's folder must be in a git checkout, and `git` must be on your `PATH`. Otherwise, and for
a ref git does not know, the run stops with exit status `2`.

# nvs test --mutate

    nvs test <paths>... --mutate[=<target>,...] [--since <ref>] [--filter <text>]
                        [--format human|json] [--jobs <n>]

`--mutate` checks whether your tests notice a wrong line. A line can run in a test while no
assertion checks its result. Coverage counts that line as tested. A mutation run finds it.

The run has two steps:

1. The tests run once, with coverage. If a test fails here, no mutant runs and the run fails.
2. For each place in the program that can change, Novis builds a copy with that one change. This
   copy is a **mutant**. The tests that ran the changed line run again against it.

These are the changes Novis makes:

| Change | Example |
|---|---|
| a comparison moves its boundary | `<` to `<=`, `>=` to `>` |
| an equality test is reversed | `==` to `!=` |
| an arithmetic operator is swapped | `+` to `-`, `*` to `/` |
| a logical operator is swapped | `&&` to `\|\|` |
| a boolean value is flipped | `true` to `false` |
| an `if` or loop condition is reversed | `if ($ok)` runs as `if (!$ok)` |

Each change keeps the types of the code, so every mutant compiles. A test method, and every line
inside it, never changes. A change that would give a value the wrong type is not made: `*` and `/`
swap only on `float` and `decimal` values.

Some conditions tell Novis the type of a value inside the `if`. These conditions are not changed:

- an `is` test, such as `$x is int`
- an `==` or `!=` that compares a value with `null`, a literal or a constant, when the value can
  have more than one type. `?int`, `int|string` and `mixed` are such types.

`if ($n == 10)` is changed when `$n` is an `int`. `if ($n == null)` is not changed when `$n` is a
`?int`.

Some code changes only a message, never a result. A value inside the arguments of a `Core\Log`
method makes no mutant. A value inside the arguments of the `new` in a `throw new` makes none
either. Output written with `echo` or `Core\Html` makes mutants, because the page is the result of
a web program.

**Choosing what changes.** `--mutate` alone changes the whole program. `--mutate=<target>` changes
only the code you name. The `=` is required. Separate two targets with a comma:

| Target | Changes |
|---|---|
| `--mutate=src/Shop` | every file in that folder; a file path works the same way |
| `--mutate=Shop\Cart` | the methods of that class |
| `--mutate=Shop\Cart::total` | that one method |

A path is relative to the current folder. A target that matches nothing stops the run with exit
status `2`, and the error names it. `--since <ref>` changes only the lines you added or edited since
a git branch, tag or commit. It also changes every line of a new file that git does not track yet.
Under `--mutate`, `--since` chooses only the mutants, and every test runs. With a target as well,
a mutant must be in the target and on a changed line.

`#[NoMutate(because: '...')]` on a class or a method keeps its code out of every mutation run. The
attribute is `Core\Test\NoMutate`. `because:` says why, and it must not be empty. The attribute
does nothing outside `--mutate`.

```nvs
<?nvs
use Core\Test\NoMutate;

#[NoMutate(because: 'only the text of a label')]
final class PriceLabel
{
    public static function show(int $cents): string
    {
        return $cents >= 10000 ? 'big order' : 'order';
    }
}
```

Each mutant has one of these results:

| Result | Meaning |
|---|---|
| `killed` | a test failed, so the tests noticed the change |
| `survived` | every test passed, so no test checks this line's result |
| `timed out` | a test ran past its time limit, which also counts as noticed |
| `not reached` | no test ran the line, so there was no test to run |
| `equivalent` | the changed method compiles to the same machine code as the original, so no test can notice it |
| `duplicate` | the changed method compiles to the same machine code as an earlier mutant of that method |

An `equivalent` or `duplicate` mutant runs no test. Novis compiles the changed method on its own
and compares the code before any test runs. It finds only the changes the compiler removes, so some
equivalent mutants still survive.

The **mutation score** is the `killed` and `timed out` mutants as a percentage of the reached
ones. `not reached`, `equivalent` and `duplicate` mutants do not count in it. A changed loop condition can make a loop run forever. A test can have no time limit from
`#[Test(timeout: ...)]` or `[limits] wall_time`. Under `--mutate`, such a test gets a limit of ten
times the slowest test's time, plus one second.

**Every mutant has a key**, such as ``Shop\Cart::total#2: `<` to `<=` ``. The key is the method,
then a number, then the change. The number counts that method's mutants with the same change, from
1, in the order they are written. A key stays the same when you edit code outside the method. It
changes when you rename the method.

Under `human`, the run prints the normal test report, then the counts and the score. Then it lists
the survived mutants by method, the method with the most first. Each source line is one line: its
first change, how many more changes on that line survived, and the key. The `compiles:` line says
how many times the program was compiled for the mutants:

```text
  compiles: 1, for the 5 mutants that ran tests

  mutants: 3 killed, 2 survived, 0 timed out, 1 not reached
  skipped: 1 equivalent, 0 duplicate (compiled to the same code, so no test ran)
  mutation score: 60.0% (3 of 5 reached mutants made a test fail or time out)
  survived in Shop\Cart::total (2):
    src/Shop/Cart.nvs:14  `<` to `<=` (+1 more on this line)  mutant = "Shop\\Cart::total#1: `<` to `<=`"
```

**Survivors you have checked go in `mutants.toml`.** Some survivors are fine, for example a change
that gives the same result. Write each one in a file named `mutants.toml` in the program's folder.
That is the folder of `nvs.toml`, or the entry file's folder when there is no `nvs.toml`. The
`mutant = ...` part of a report line is ready to paste:

```toml
[[judged]]
mutant = "Shop\\Cart::total#2: `<` to `<=`"
reason = "the loop ends on the same item either way"
equivalent = true
```

`reason` is required, and it must not be empty. `equivalent` is `false` when you leave it out. A
survivor with an entry is not listed. The report counts it on a `judged:` line. A judged survivor
with `equivalent = true` does not count in the score. Every other judged survivor still counts as
survived. An entry that matches no mutant prints a warning with its key, and the exit status does
not change. A file that cannot be read, an unknown key or one key judged twice stops the run with
exit status `2`. No command writes this file: a person checks each survivor.

Under `json`, `schemaVersion` is `5`. The document has a `mutants` array with one `{file, line,
change, outcome, tests, function, key}` record for each mutant. `tests` is the number of tests that
ran the mutant's line. A `duplicate` record also has `sameAs`, the key of the earlier mutant. A
judged survivor's record has `judged: true`, its `reason` and `equivalent`. A survived mutant does
not make the run fail, so the exit status is the one of the first step.

**A run compiles the program once.** Novis compiles every mutant into one copy of the program. Each
changed place has its original code and its changed code, and a number chooses which one runs. Before
each mutant's tests, the run sets the number to that mutant. So a run costs one compile, plus a few
tests per mutant. The mutants run one at a time, and each mutant's tests run across `--jobs` cores. `--mutate` runs a
program's `#[Test]` methods only. `--format junit`, `--list`, `--update`, `--watch` and a `.nvst`
tree are each an error beside it.

# nvs debug

    nvs debug [--once] [--tail <file>]... [--listen <address>]

With `[debug] store = true` in `nvs.toml`, every request is recorded. The records are written to
files in `debug/spool` in the data folder. `nvs debug` reads those files into one SQLite database,
`debug/index.sqlite`, and deletes each file once it is read. Then it waits for new files.

You usually do not start it yourself. A `nvs serve` in development starts `nvs debug` when
`store` is on, and stops it when the server stops. A production server never starts it. On a
production host, you run `nvs debug` yourself, when and where you choose.

| Option | What it does |
|---|---|
| `--once` | read the files that are ready, delete old records, then exit |
| `--tail <file>` | also read this log file, and each line added to it later; you can give it more than once |

A `--tail` file must use the `json` log format (`[log] format = "json"`). `nvs debug` never changes
that file. It remembers how far it read, so a restart goes on from there. When the file is rotated,
it first reads the rest of `<file>.1`.

Only one `nvs debug` runs for a data folder. A second one prints an error and exits with status
`1`. A server that finds one running uses it.

The index deletes a request's records once they are older than `[debug] index_age` (`7d`). When it
grows past `[debug] index_max` (`1G`), the oldest requests are deleted first. A line that cannot be
read is kept as an error record, and the lines after it are read as usual. A file that cannot be
read is tried again later, and the other files are read in the meantime.

**What a record contains.** In development, a record has the request's method, path, query,
headers and body, and the response's status, headers and body. The values of `Authorization`,
`Cookie`, `Set-Cookie` and `Proxy-Authorization` are replaced with `[redacted]`. A body is stored
as the program read it, up to `[limits] request_body`. A longer body is cut at that size. A body the
program did not read is not stored. A program can read a body into a class with
`Core\Request::jsonAs<X>()` or `formFieldsAs<X>()`. The decoder fills each `secret` property of `X`, and
the stored body has `[redacted]` in place of that value. If Novis cannot find those values in the
body, the body is not stored. That is the case for a multipart form, for example. A body the program
read another way is stored as it was read, so a password in it is in the index. The files are in a
folder that only your account can open. In production, a record has no headers and no body.

# The debug viewer

    nvs debug [--listen <address>]

`nvs debug` also serves the debug viewer, a page that shows the recorded requests in a browser. It
prints a link to the page when it starts. Open that link, because it contains a token. The viewer
answers only a browser that has the token. Another account on the same computer does not have it,
so it cannot read your requests. The address is `127.0.0.1:8137` unless `[debug] listen` or
`--listen` sets another one. If that port is in use, `nvs debug` stops with an
error. It does not pick another port, so a link to the viewer still works after a restart.

| Page | What it shows |
|---|---|
| `/` | the requests, newest first: method, route, status, duration, queries, dumps and exceptions |
| `/requests/<trace id>/<span id>` | one request: its timeline, its queries, dumps and log lines, and its input |
| `/stream` | every record of every request, as one list you can filter |
| `/log` | the log lines |
| `/search` | a full-text search over requests, dumps, logs, queries and exceptions |
| `/exceptions` | one row for each exception class and the line that threw it, with a count |
| `/compare/...` | two requests side by side |

New rows appear at the top while you watch. Press `p` or scroll down to pause the list. The list
then counts the new rows and does not move. Each page and each filter has its own link, which still
works after a reload. You can paste a trace id, or the value of a `traceresponse` header, into the
search box. The viewer then opens that request.

Every key is listed when you press `?`. `j` and `k` move the selection, `Enter` opens it, `/` goes to
the search box and `s` saves a filter. `m` marks a request, and the next request you open is
compared with it. `c` compares a request with the previous request to the same route.

`r` replays a request: it sends the recorded request to your development server again. The handler
runs again, with all its side effects, so the viewer asks first. Replay works only when `nvs serve`
started `nvs debug`. `t` makes a test from a request, the same test `nvs debug to-test` prints.

Every `file:line` in the viewer is a link that opens the file in your editor. Pick the editor in
the **Editor** dialog. The choices are `vscode` (the default), `vscode-insiders`, `vscodium`,
`cursor`, `zed`, `idea`, `phpstorm` and `custom`. With `custom`, you write the link yourself, with
`{file}`, `{line}` and `{column}` in it. A path mapping replaces the start of a recorded path. Use
one when the program ran in a container and its paths differ from yours. The viewer saves these
settings and your saved filters next to the index.

A profile that `Core\Debug::startProfile` wrote has a download link. In VS Code, it also has a link
that opens it in the Novis extension. When the server ran with `[debug] mode = ["coverage"]`, a
request's page shows which lines of each source file ran, and how often.

The viewer is for you alone. It answers only a browser that opened the printed link, which contains
a token. `nvs debug` creates the token once, in `debug/` in the data folder, so the link still works
after a restart. A page on another site cannot read the viewer. When `listen` is not a loopback
address, for example in a container, `nvs debug` serves the viewer over HTTPS. It creates a
certificate for that. Your browser warns about the certificate once. `[debug] tls_cert_file`,
`tls_key_file` and `token_file` name your own files instead.

# nvs debug to-test

    nvs debug to-test <trace id> [--span <span id>] [--write <file>]

`nvs debug to-test` prints a test made from one recorded request. The test sends the same request
to your program with `Core\Test::request`, and checks that the response is the same. Save it in your
project and run it with `nvs test .`.

The test checks the status, the `Content-Type` header, the `Location` header of a redirect, and the
body. A JSON body is read with `json()` and compared with the recorded JSON as data, so spaces do
not matter. The keys must be in the same order. Any other body is compared byte for byte. Headers
such as `Date` change with every request, so the test does not check them.

| Option | What it does |
|---|---|
| `--span <span id>` | choose one request when several requests have the same trace id |
| `--write <file>` | write the test to a new file; if the file exists, nothing is written |

A header the record has as `[redacted]`, such as `Authorization`, gets the value `'redacted'`, with a
comment above it. Put a test credential there. If the recorded request body is not complete, no
test is printed, because part of a body is a different request. If the recorded response body is
not complete, the test checks the status and the headers only.

The request needs its input in the index, so it must come from a development server with
`[debug] store = true`. The viewer's `t` key shows the same test, with a button that copies it.

# nvs debug link

    nvs debug link [--json] [--listen <address>]

`nvs debug link` prints the link to the debug viewer. The link contains the token, so it opens the
viewer. The command starts nothing. It reads the same settings and the same token file as
`nvs debug`, so the link works while `nvs debug` runs. If the token file does not exist yet, the
command creates it, as `nvs debug` would.

| Option | What it does |
|---|---|
| `--json` | print one JSON object with `url`, `embed`, `running` and `recording` |
| `--listen <address>` | use this address, the one `nvs debug --listen` was given, in place of `[debug] listen` |

`embed` is the link an editor uses to show the viewer inside its own window. `running` is `true`
when `nvs debug` runs for the data folder. `recording` is the value of `[debug] store`. The
command prints a note on standard error when no `nvs debug` runs. It also prints a
note when `[debug] store` is off, because then no request is recorded.

If `[debug] listen` has the port `0`, `nvs debug` picks the port when it starts. Then the command
prints an error, and you use the link that `nvs debug` printed.

# Profiling

    nvs run <file> --profile callgrind|speedscope [--profile-out <file>]
    nvs test <paths>... --profile callgrind|speedscope [--profile-out <file>]

`--profile` measures how long each function call takes. When the program or the tests end, it
writes the result to a file. You do not change the program for this.

| Format | The file | Open it with |
|---|---|---|
| `callgrind` | `callgrind.out.<pid>` | KCachegrind or Webgrind |
| `speedscope` | `profile.<pid>.speedscope.json` | speedscope, or the *View Profile* command in VS Code |

`<pid>` is the process ID of the run. The file is written in the current folder.
`--profile-out <file>` writes it to another file instead. At the end, the run prints
`profile: <path>` on standard error.

A Callgrind file has totals for each function. One total is the time in the function itself. The
other includes the time of the functions it called. The file also lists who called each function.
A speedscope file has every call on a timeline, in the order the calls ran.

Under `nvs test`, each test has its own timeline in a speedscope file. A program that `spawn`
starts also has its own timeline. A Callgrind file adds the totals of all of them together. The
time a program waits for a spawned program is also in that sum, so the total can be larger than
the time the run took.

The timing adds a cost to every call. In a speedscope profile, it is about 170 nanoseconds per call
on a current desktop processor. Every call pays the same cost, so code with many small calls looks
slower in a profile than it is. A `.nvst` tree has no
profile. On `nvs run`, `--profile` is an error beside `--dump-ir`, `--dump-asm` or `--count`.

A program can also profile one part of itself. `Core\Debug::startProfile` and
`Core\Debug::stopProfile` write a speedscope file around the code between them. The configuration
must allow this. `[debug] mode` must include `"profile"`, and `[capabilities.debug] profile` must
allow the file's folder. `--profile` needs neither of them, because only the person who runs the
command can add the flag.

# nvs build --compile

    nvs build --compile <file> [-o <path>]

Writes a **portable single-file executable**: a copy of the `nvs` binary itself with the program's
source appended — the entry file plus every file its `require` graph reaches, as a flat list of
paths and bytes, followed by a small footer. No compilation result is stored; the bundle compiles
its source when it starts, exactly as `nvs run` would.

```text
$ nvs build --compile report.nvs -o disk-report
wrote disk-report (1 source file(s))
$ ./disk-report
uploads: 420 MB
total: 420 MB
```

- `-o <path>` says where to write it; the default is the entry file's stem in the current directory
  (`routes.exe` on Windows, `routes` elsewhere). The command reports `wrote <path> (N source
  file(s))`.
- Running the bundle runs the program. **Its whole command line belongs to the program**: a bundle
  never interprets `run`, `check`, `--help` or any other `nvs` argument, so an application whose
  first argument happens to be `run` keeps it.
- A bundle reads `./nvs.toml` from the directory it is *run in*. If there is none, it reads
  `nvs.toml` in the `.nvsdata` folder next to the bundle. A file with an error stops the run. A
  bundle never writes a `nvs.toml`. Ship the configuration beside it, or run it from a directory
  that has none.
- Files reached only through `autoload` at run time, or opened with `Core\IO`, are not in the
  bundle: it carries the static `require` graph and nothing else.
- Each `.nvsx` file that the build's configuration lists under `[[extension]]` goes into the
  bundle, with its `sha256` and its `grants`. The bundle checks each `sha256` when it starts and
  loads these extensions before the ones in the configuration file it reads. A file that does not
  match its `sha256` stops the build.
- On Windows, the executable shows the program's own name, version and icon in Explorer and in
  Task Manager. They come from the nearest `package.toml`, found from the current directory: the
  package's `name` and `version`, and the `.ico` file `[bundle] icon` names. Without a
  `package.toml`, the executable shows the icon and version of `nvs`.

# nvs build --openapi

    nvs build --openapi <file>

Writes an OpenAPI 3.1 document for the program's `#[Route]` methods to standard output: `openapi:
"3.1.0"`, an `info` block whose `title` is the entry file's stem and whose `version` is the nearest
`package.toml`'s `version` (`0.0.0` without one), and one `paths` entry per route
with its method, an `operationId` of `Class.method`, its path parameters with their schemas, and
its responses. Nothing runs. `nvs build` with neither `--compile` nor `--openapi` is refused rather
than doing nothing. The output below, for a program with one route, is trimmed at the `...`.

```text
$ nvs build --openapi orders.nvs > openapi.json
$ cat openapi.json
{
  "info": {
    "title": "orders",
    "version": "0.0.0"
  },
  "openapi": "3.1.0",
  "paths": {
    "/orders": {
      "get": {
        "operationId": "Orders.index",
        ...
        "summary": "Lists every open order."
      }
    }
  }
}
```

# nvs api diff

    nvs api diff <old.json> <new.json>

Reads two OpenAPI documents — typically the last release's and the one `nvs build --openapi` just
produced — and prints one line per change, classified `breaking`, `additive` or `cosmetic`, then a
summary; `no change` when they agree. It exits `1` when any change is breaking, so it is a CI gate:

```text
breaking: paths./users/{id} — path removed, with every operation on it
additive: paths./users.get.query.q — optional parameter added
2 change(s): 1 breaking, 1 additive, 0 cosmetic
```

# nvs config check and nvs config dump

    nvs config check [files]... [--as <account>]
    nvs config dump [files]... [--origin] [--toml]

Both read the configuration tree offline and run nothing, so a tree can be validated in CI before
it is deployed. With no files named they read `./nvs.toml`. If there is none, they read `nvs.toml` in
the data folder. If there is neither, they use the default settings, which is not an error.
Naming files positionally is the same as `--config`. Neither command creates the data folder.

- `config check` resolves the whole tree — includes, `[[app]]` blocks, secrets — and prints one
  line: `ok: 2 files, 4 directives set, 1 override, 0 warnings`. It exits `1` on anything the tree
  refuses: a TOML syntax error, an unknown key (`E0601`), a missing or cyclic include (`E0605`,
  `E0606`), an `[[app]]` block naming both `root` and `entry` or neither (`E0609`), a secret with
  both `password` and `password_file` (`E0608`), and a `[limits]` value that is not a valid
  quantity or is above its `[limits.hard]` ceiling (`E0601`).
- `config check --as <account>` also checks who owns each file and who can write it. It checks the
  files for `<account>`, the account that will run `nvs serve`. A file that `nvs serve` would not
  read as that account is `E0607`, with the same help `nvs serve` prints. On Unix, `<account>` is a
  user name or a uid. On Windows, it is an account name, `LocalSystem` or `NT SERVICE\<service>`.
  An account that does not exist on this machine is `E0662`. The check reads the files on this
  machine as they are now. Without `--as`, `config check` does not check ownership. `config dump`
  does not take `--as`.
- `config dump` prints every key in force, one per line in dotted-key order, with array-of-tables
  blocks numbered (`app.0.root`, `include.1.path`). The list has each key that a file sets. It also
  has the default value of every key that no file sets, as `nvs serve` uses it. A value or a table
  name can have a new line, a control character or a right-to-left mark. The dump writes it as a
  TOML escape, such as `\n` for a new line. Each key stays on one line, and you can see text that a
  terminal would hide.
- `config dump --origin` adds where each value comes from: the file that sets the key, and the file
  it replaced. A default value shows `default`. When the mode chooses the value, it shows
  `default (production)` or `default (development)`.
- `config dump --toml` prints the keys that the files set as one TOML document. The document has no
  default values, so it reads back as the same configuration. You can compare the documents of two
  environments.

```text
$ nvs config dump --origin
app.0.capabilities.script.spawn = true                    /srv/shop/nvs.toml
app.0.origin                    = "https://example.test"  /srv/shop/nvs.toml
app.0.root                      = "."                     /srv/shop/nvs.toml
limits.hard.memory              = "512M"                  /srv/shop/nvs.toml
limits.memory                   = "256M"                  /srv/shop/nvs.toml
log.format                      = "json"                  default (production)
log.target                      = "stderr"                default
```

The real list is longer: it has every default. `log.target` shows where the log records go.

# nvs ext new and nvs ext build

    nvs ext new --lang rust|c <dir>
    nvs ext build [project]

An extension is a `.nvsx` file. It is a WebAssembly component that adds one class to Novis. You
write the code in Rust, C or another language that compiles to WebAssembly. `nvs ext` is the tool
for the steps around your compiler. It has six commands: `new`, `build`, `inspect`, `test`,
`verify` and `pin`.

`nvs ext new` creates a project in `<dir>`. The folder must not exist, or must be empty. The project
has an `nvsx.toml`, the WIT files of the `nvs:ext` world under `wit/deps/`, the code of one
function, a test for it, and a README that says how to build it. `--lang rust` builds with `cargo`
for the target `wasm32-wasip2`. `--lang c` builds with wasi-sdk and `wit-bindgen`, which you
install yourself.

```text
$ nvs ext new --lang rust greeting
greeting
$ cd greeting
$ cargo build --release --target wasm32-wasip2
$ nvs ext build
greeting.nvsx
sha256 = "9f2c...e41a"
```

`nvs ext build` makes the `.nvsx` file from the wasm file your compiler wrote. The project folder
is the working directory unless you name one.

- `nvsx.toml` describes the class: its name, the WIT interface it exports, the path of the wasm
  file, and each method's signature and help. A signature is written in Novis, such as
  `distanceKm(string $from, string $to): float`.
- The wasm file may be a core module or a component. A WASI preview 1 module does not build. Build
  for `wasm32-wasip2` instead.
- The `.nvs` files in the project's `source` folder go into the `.nvsx` file too. One `sha256`
  covers the code and the source.
- The command checks that the result loads, and only then writes it. The file is written beside
  `nvsx.toml`. Its name is the last part of the class name: `Shop\GeoTools` gives `geo-tools.nvsx`.
  The command prints the path and the `sha256` line for your configuration.
- The same project always gives the same bytes.

# nvs ext inspect, nvs ext verify and nvs ext pin

    nvs ext inspect <file> [--source]
    nvs ext verify <file>
    nvs ext pin <file>

These three commands read a `.nvsx` file. Use them before you load a file that somebody else
built. None of them runs code from the file.

- `nvs ext inspect` prints the class, the interface, the world version and the `sha256`. Under
  `requests` it prints the folders the extension wants to read or write and the hosts it wants to
  connect to. Then it prints each method's signature and help, the constants, the settings block
  and the paths of the Novis source files. `--source` also prints the text of each source file. It
  works for a file that does not load, too. Control characters in the file's text are printed as
  `\u{..}`, so the file cannot change your terminal.
- `nvs ext verify` runs every check that loading the file runs, and prints
  `<file>: passes every check loading it runs`. If a check fails, it prints the error and exits with
  status `1`.
- `nvs ext pin` runs the same checks. Then it prints the `[[extension]]` entry for the file, ready
  to copy into `nvs.toml`. `path` is the absolute path of the file. The entry has no `grants`. You
  add the folders and hosts you allow yourself, after you read `nvs ext inspect`.

```text
$ nvs ext pin greeting.nvsx
[[extension]]
path   = "/home/dev/greeting/greeting.nvsx"
sha256 = "9f2c...e41a"
```

# nvs ext test

    nvs ext test [project] [--config <file>]

Runs the `#[Test]` methods of every `.nvs` file in the project's `tests` folder, as `nvs test` does.
The `.nvsx` file that `nvs ext build` wrote is loaded, so the tests can call its class.

```text
$ nvs ext build
greeting.nvsx
sha256 = "9f2c...e41a"
$ nvs ext test
  GreetingTest
    ✓ helloGreetsByName                0.4 ms

  0 failed, 1 passed, 0 skipped, 0 flaky in 1 ms
```

- The command does not read `./nvs.toml` or the `nvs.toml` in the data folder. Without `--config`,
  the extension gets no folders and no hosts. Name a configuration with `--config` to test with the
  grants it gives.
- If the configuration lists the `.nvsx` file with another `sha256`, the command stops with an
  error that names both values.
- If a file of the project changed after the last `nvs ext build`, the command stops with an error.
  Run `nvs ext build` again first.

# nvs info

    nvs info [--licenses]

Prints what this binary is: the version, commit, build profile, target triple, Rust and Cranelift
versions under **Build**; the operating system, architecture, CPU parallelism and executable path
under **Host**; and under **Licensing** the `nvs` license (MIT) followed by every third-party
component with its version and the license Novis takes it under. `--licenses` appends every
license text in full. There is no `nvs -i`: that is an
unknown argument, and `nvs` exits with status 2. The output below is trimmed at each `...`.

```text
Novis 0.0.1
==============================================================================

Build
  version             0.0.1
  commit              932147445
  profile             debug
  target              x86_64-pc-windows-msvc
  compiler            rustc 1.97.1 (8bab26f4f 2026-07-14)
  code generator      Cranelift 0.135.3
  ...

Host
  operating system    windows
  architecture        x86_64
  cpu parallelism     16
  ...

Licensing
  nvs                 MIT
  third-party         listed below
  license texts       run `nvs info --licenses`

COMPONENTS (339)
...
```

# nvs telemetry

    nvs telemetry on|off|status|reset|show|upload

Telemetry counts how often each `nvs` command runs. It is off until you run `nvs telemetry on`. The
counters are in a plain file in the data folder (see `--data`), and you can read it with any editor.

Telemetry and `nvs update-check` are two separate settings. Turning one on does not turn on the
other.

| Command | What it does |
|---|---|
| `nvs telemetry on` | Starts recording. Each `nvs` command after it adds 1 to its counter. |
| `nvs telemetry off` | Stops recording. The counters already recorded stay in the file. |
| `nvs telemetry status` | Prints whether telemetry is on, the path of the counter file, the upload address, the last and next upload, and how many uploads were accepted. |
| `nvs telemetry reset` | Deletes the `telemetry` folder in the data folder. Telemetry is then off. |
| `nvs telemetry show` | Prints the exact JSON document that the next upload sends. |
| `nvs telemetry upload` | Sends the last finished week, if it is due. It prints one line that says what happened. |

Nothing is uploaded unless `NVS_TELEMETRY_URL` is set. Deleting the `telemetry` folder in the data
folder does the same as `nvs telemetry reset`.

**`DO_NOT_TRACK` and `CI`.** If either variable is set to a value that is not empty, telemetry is off
for that command. Nothing is recorded and nothing is uploaded. Your setting is not changed, and
`nvs telemetry status` names the variable. Most CI services set `CI`.

**The counters.** Each command adds 1 to one counter:

- `invocation.<command>` counts one run of a command, such as `invocation.run` or
  `invocation.serve`. The name of the command comes from the list of commands that `nvs` has.
- `invocation.unknown` counts a command line that runs no command. This includes a mistyped command,
  `--help` and `--version`. Your text never becomes a counter name.
- `internal-error.<command>` counts a crash of `nvs` itself during that command. It is counted once.
  The error message is not recorded.
- `internal-error.unknown` is the same for a command line that runs no command.

An error in your program is not an internal error, and it is not counted. `nvs serve` adds 1 to
`invocation.serve` when it starts. Nothing about requests is counted.

**The upload.** An upload sends one JSON document for one finished week:

```json
{"schema":1,"version":"0.4.0","os":"linux","arch":"x86_64","week":"2026-W42","first":false,"days_active":3,"counters":{"invocation.run":41,"invocation.serve":2,"internal-error.check":1}}
```

| Field | What it is |
|---|---|
| `schema` | The version of this format. It is `1`. |
| `version` | The version of `nvs`. |
| `os` | `linux`, `macos`, `windows` or `other`. |
| `arch` | `x86_64`, `aarch64` or `other`. |
| `week` | The week the counters are for, as an ISO week in UTC. A week starts on Monday. |
| `first` | `true` until one upload from this data folder is accepted. |
| `days_active` | On how many days of that week `nvs` ran, from 1 to 7. |
| `counters` | Each counter that is above zero. |

The document contains no machine ID, no time of day, no arguments, no paths and no environment
variables.

**When an upload is sent.** Only `nvs update-check` and `nvs telemetry upload` send an upload. No
other command sends one. `nvs serve` never sends one, however long it runs. To send your counters,
run one of the two commands from a scheduled job, such as a daily cron job.

An upload sends the last finished week, once. It waits until a random time in the first 24 hours
after the week ends, so that many machines do not send at the same moment. Before that time, the
command prints when the week can be sent. A week with no counters is not sent.

An upload has 3 seconds. If it fails, the command still does its own work and exits with the same
status. The upload goes to the collector (the server at `NVS_TELEMETRY_URL`), and the command prints
one of these lines:

| Line | What happens to the week |
|---|---|
| `telemetry: sent week 2026-W43, 180 bytes` | The collector accepted it. It is removed from the file and added to the totals. |
| `telemetry: could not send (timeout after 3s); kept for next time` | It stays in the file, and the next upload sends it. A status `429`, a `5xx` or any other status does the same. |
| `telemetry: refused by the collector (400: week); dropped` | It is removed from the file. A status `413` or `415` does the same. |

The upload is one `POST` of the document as `application/json`. It follows no redirect.

**The collector.** The collector is a Novis web app in the folder `apps/telemetry/` of the Novis
repository. Its `README.md` shows how to run one on your own machine and how to deploy it. It returns
one of these statuses, and the command does this with the week:

| Status | Meaning | What happens to the week |
|---|---|---|
| `204` | The collector added the week to its totals. | It is removed from the file. |
| `400` | A field of the document is not valid. | It is removed from the file. |
| `413` | The document is larger than 4 KiB. | It is removed from the file. |
| `415` | The document is not JSON. | It is removed from the file. |
| `429` | This address sent too many reports in the last hour. | It stays in the file. |
| any other | The collector did not handle the document. | It stays in the file. |

The collector saves no report and no address. Every total it has is public on its pages, and as JSON
and CSV.

`nvs telemetry show` prints the document on standard output. If the week is not finished yet, a note
on standard error says so. The counter file looks like this:

```text
# Usage counters of `nvs telemetry`. Run `nvs telemetry show` to see what an upload sends.
uploads 0
bytes 0
week 2026-W43
days 0000100
invocation.run 3
invocation.telemetry 1
```

`days` has one digit for each day from Monday to Sunday. `1` means that `nvs` ran on that day.

After an upload is accepted, the file also has `last`, the time of that upload. While a finished week
waits, the file has `pending` with that week's lines, and `due`, the time the week can be sent. Both
times are seconds since 1970-01-01 in UTC.

# nvs update-check

    nvs update-check [--json]

`nvs update-check` checks whether a newer version of Novis is released. It prints whether telemetry
is on, the version you run and the latest released version. It reads the latest release from
GitHub. `NVS_UPDATE_URL` replaces that address.

Each line is printed as soon as its step is done, so you can see where a slow network waits. The
command always exits with status `0`. A script uses `--json`, which prints one JSON document:

```json
{"schema":1,"telemetry":"off","telemetry_url":null,"local":"0.4.0","latest":"0.5.0","update_available":true,"error":null,"upload":null,"last_upload":null,"next_upload":null,"uploads":null,"bytes_sent":null}
```

`latest` and `update_available` are `null` if the release could not be read, and `error` then says
why. When telemetry is on, the command also sends the last finished week, if it is due (see
`nvs telemetry`). It then prints the upload line, the time of the last and the next upload, and the
totals. In the JSON document, `upload` is `{"result":"sent","message":"telemetry: sent week …"}`. Its
`result` is `sent`, `kept`, `dropped`, `not-due` or `nothing`. `upload` and the four fields after it
are `null` while telemetry is off.

# nvs meta --json

    nvs meta --json [entry.nvs]

Prints the built-in `Core` classes as one JSON object. The `Core` reference on this website is
made from the same data. `--json` is required. The object has eight top-level keys:

- `classes`: one object per `Core` class, with `name` and `members`. A method has `name`, `kind`
  (`static` or `instance`) and `signature` (the whole declaration, such as `length(string $s): uint`).
  It also has `params`, `names`, `returns` and `doc`. Each parameter has `name`, `type` and `qualifier`. The
  qualifier is `neutral`, or the tainted or secret value the parameter accepts. `doc` has `short`,
  a `params` list with a `desc` per parameter, `return`, and `errors` where the method throws. A
  class with constants has a `constants` list.
- `enums`: `name` plus `doc` with `short` and one `cases` entry per case.
- `exceptions`: every error class, with `name`, `parent`, and the `properties` a class adds.
- `interfaces`: the global interfaces, with `typeParams` where they have any.
- `attributes`: the compiler-recognized attribute names.
- `directives`: every `nvs.toml` directive with its `key` and when a change applies (`Reload` or
  `Boot`).
- `settable`: every key a request may change with `Core\Config::set`, with its `key` and `how`
  (`up-to-ceiling`, or `off-only` for a key a request may only set to `false`).
- `capabilities`: one row for each `Core` method that needs a capability, with `class`, `member`
  and `capability`. To show a method's capability, find its row by `class` and `member`. A method
  with no row needs no capability.

With an entry file, the command also reads that program and adds a ninth key, `program`. It has
the program's own `classes`, `enums` and `types` (its `type` aliases) in the same form. It also has
`schemas` when a class's JSON schema refers to another class. A declaration's `doc` is the `///`
comment above it: its text as `short`, and its `@see` and `@example` tags as lists. The other
eight keys are exactly the same as without the entry file. For a file with one documented class:

```text
$ nvs meta --json cart.nvs
…,"program":{"classes":[{"doc":{"short":"A shopping cart."},"members":[{"doc":{"short":"Adds items to the cart."},"kind":"instance","name":"add","signature":"add(uint $count = 1): uint","visibility":"public"}],"name":"Cart"}]}}
```

A class with `#[Core\Json\Derive]` also has a `json` key. It is the JSON schema of the document the
class encodes to, the same schema `nvs build --openapi` writes. It has the inherited keys, the
renamed keys, and no skipped keys. Its `required` list gives the keys in the order they are written.
A set of allowed values is a `string` or `integer` with an `enum` list. A property whose type is
another derived class is a `$ref` to `#/program/schemas/<name>`. The `program` object then has a
`schemas` map with that schema, once for each class. A class with a `toJson` method you wrote
yourself has no `json` key. For a `Post` with an inherited `id`, an allowed set and an `Author`:

```text
$ nvs meta --json shop.nvs
…,"json":{"properties":{"author":{"$ref":"#/program/schemas/Shop.Author"},"id":{"type":"integer"},"state":{"enum":["draft","live"],"type":"string"}},"required":["id","state","author"],"type":"object"},…,"schemas":{"Shop.Author":{"properties":{"name":{"type":"string"}},"required":["name"],"type":"object"}}}}
```

# nvs ast

    nvs ast [--json] [--resilient | --strict] <file>

Parses one file and prints its syntax tree in a debug notation, indented one level per node, with
spans as `file-index:start..end` byte offsets. It parses only — names are not resolved and nothing
is type-checked, so a file `nvs check` refuses may still print a tree. For a file holding
`<?nvs echo -x;`:

```text
$ nvs ast --json neg.nvs
{"children":[{"children":[],"kind":"Whitespace","span":[5,6]},{"children":[{"children":[],"kind":"Whitespace","span":[10,11]},{"children":[{"children":[],"kind":"ConstFetch","span":[12,13]}],"kind":"Unary","op":"Neg","span":[11,13]}],"kind":"Echo","span":[6,14]},{"children":[],"kind":"Whitespace","span":[14,15]}],"kind":"File","span":[0,15]}
```

`--json` prints a frozen document instead of that notation, for a tool rather than a person: one
object per node, carrying `kind`, `span` as `[start, end]` byte offsets, that production's own
scalar fields — an operator, a flag, or which form a member name took — and `children`. The text of a
value written directly in the code is not one of those fields: its span names it. This command does
not type-check, so it cannot know which of those values is `secret` and needs a placeholder in
place of its bytes.

The comments and the whitespace the grammar drops are in that document too, as nodes of the same
shape — `Whitespace`, `LineComment`, `BlockComment` or `DocComment`, each with its own span and no
children — sitting under the innermost node whose span contains them, so a comment between two
methods is a child of the class. A consumer wanting only the grammar's own nodes filters on `kind`.

`--resilient` is the default: the tree the parser recovered into is printed whatever it reported,
which is what makes the output useful on the file that does not compile. `--strict` prints no tree
at all once an error is reported. The two are refused together.

# nvs fmt

    nvs fmt [--check | --diff | --stdin] [paths]...

`nvs fmt` rewrites each file in one fixed layout. A body is indented by four spaces. Each statement
and each closing brace is on its own line. A plain string uses single quotes. The command has no style
options and reads no setting, so every project gets the same layout. Running it on a formatted file
changes nothing.

`nvs fmt` never moves a line break that you wrote. A list or a chain of calls stays on one line, or
it gets one item per line. There is no line width.

A path can be a file or a directory. In a directory, `nvs fmt` formats every `.nvs` file under it. It
skips what `.gitignore` names, every folder named `vendor` or `target`, and every folder whose name
starts with a dot. A file or folder that you name yourself is always formatted.

By default, each file is written back in place. A file that is already formatted is not written, so its
modification time stays the same. Two options write nothing and only report:

- `--check` prints the name of each file that would change.
- `--diff` prints a unified diff for each file that would change.

```text
$ nvs fmt --check cart.nvs order.nvs
order.nvs
```

`--stdin` formats what it reads from standard input and prints the result. An editor uses it to format
a file that is not saved yet. With `--check`, the name it prints is `<stdin>`.

A file with a syntax error is not formatted. `nvs fmt` prints its name and the errors on standard
error, leaves the file as it was, and goes on with the next file. With `--stdin`, it prints nothing on
standard output.

The exit status says what the run found:

| Status | Meaning |
|---|---|
| `0` | Nothing to change, or every file was formatted. |
| `1` | With `--check` or `--diff`: a file would change. |
| `2` | A file has a syntax error, or could not be read or written. |

When one run finds both, the status is `2`.

# nvs code

    nvs code show [--json] <symbol>
    nvs code refs [--json] <symbol>
    nvs code impls [--json] <symbol>
    nvs code outline [--json] <file>
    nvs code types [--json] <file>[:<line>[-<line>]]
    nvs code rename [--dry-run] [--json] <symbol|path:line:column> <new-name>
    nvs code move [--dry-run] [--json] <from> <to>
    nvs code order [--check] [--dry-run] [--json] [paths]...
    nvs code find [--json] <text>
    nvs code calls [--json] <method>...
    nvs code reach [--json] <method>...

Prints information about the code of the project you are in. It uses the same index as the editor.
An index is a list of every name in the project and every place it is used. The project is the folder of the nearest `nvs.toml` at or above the working directory, or the working directory
when there is none. Every `.nvs` file in it is read. Only `rename`, `move` and `order` change
files.

## The index file

The editor and `nvs code` share one index file for each project. It is in the data folder, the
folder where `nvs` keeps its own files. A call loads the index file. Then it compares the content of
every file with what the index has. It also compares each `nvs.toml` and each `.nvsx` extension file
the project uses. Only the files that changed are analysed again. So the result is the same as a
full analysis gives. A new file is analysed, and a deleted file leaves
the index. When a settings file or an extension changed, the whole project is analysed again.

After it analysed anything, the call saves the index file again, and the editor's next start uses
it. Without a data folder, the call analyses the whole project and saves nothing. The other
commands, such as `nvs check` and `nvs run`, never read this file.

## The verbs

- `show` prints where a name is declared, its signature and its `///` comment. For a `Core` name it
  prints the card `nvs agent show` prints.
- `refs` prints every place a name is written: the declaration first, then each `use` line and each
  use, in file and line order. A `Core` name lists the places the project uses it.
- `impls` prints the types that extend or implement a type, or the methods that override a method.
- `outline` prints what a file declares, one line each, with the constants, properties and methods of a class indented under it.
- `types` prints the type of each variable a file declares with `var`. `:10` limits it to line 10,
  and `:10-20` to lines 10 to 20.
- `rename` renames a class, a method, a property, a constant, an enum case, a type alias or a
  variable everywhere it is used. It is the rename the editor does. A local variable has no full
  name, so you give its position. A class alone in a file with its name also gets a new file name.
  A type alias that a class declares has the class's name in front, as `Shop\Order::Total`. It is
  renamed inside the class and everywhere `Order::Total` is written.
- `move` moves a file or a folder and changes every `require` that names a moved file. The folder of
  `<to>` must exist, and nothing may exist at `<to>` yet.
- `order` puts the constants, properties and methods of each class and interface in the standard
  order. It is the editor's
  *Order class members* action. A path is a file or a folder, and with no path every file is
  ordered. Each changed file is also formatted, as `nvs fmt` does.
- `find` prints every class, method, property, constant and enum case whose name contains the text.
  Upper and lower case count as the same. The namespace is not part of the name, and `Core` names
  are not searched. `nvs agent find` searches them.
- `calls` prints the methods that call a method, and then the methods it calls. Each of these
  methods has a line that starts with `caller` or `callee`. Each call is on an indented line under it, with its position and
  its line of code. A test that sends a request with `Core\Test::request` calls the method that
  handles the route.
- `reach` prints the routes and the tests that reach a method through a chain of calls. A route is
  a URL your program handles. Some calls are only known when the program runs. So when there may be
  more, the last line starts with `at least`.

`calls` and `reach` need the name of a method. For any other name, they exit `1` and list the
method names that are most similar:

```text
$ nvs code reach 'Shop\Prices::base'
route  GET /cart
test   Shop\CartTest::cartLoads
```

`calls` and `reach` also take several methods in one call. Each method gets a line with its name, in
the order you gave. Its lines follow, indented by two spaces. A name you give twice prints once.

```text
$ nvs code reach 'Shop\Prices::base' 'Shop\Prices::plain'
Shop\Prices::base
  route  GET /cart
  test   Shop\CartTest::cartLoads
Shop\Prices::plain
  test   Shop\PricesTest::plainIsFive
```

If one of the names is not a method, the call prints nothing and exits `1`. On standard error, it prints
each unknown name with the method names that are most similar. With `--json`, several methods print one document
with `schemaVersion` and `methods`. `methods` is a list with the document of each method, without
its own `schemaVersion`. An unknown name prints `error` and `unknown`, a list of objects with
`symbol` and `nearest`.

`rename` and `move` save the changed files. `--dry-run` prints each change and saves nothing. If the
change is not possible, the command prints why, changes no file and exits `1`. A file that changed
after the command read it also stops the command, and no file is changed.

`order` uses this order, from the top of the class to the bottom:

1. `enum` and `type` declarations, then constants, then properties, then methods.
2. In each group, everything `static` comes first.
3. Then `public` comes before `protected`, and `protected` before `private`.
4. Then `abstract` comes before `final`, and `final` before the rest.

The constructor is the first method that is not `static`. Two that are equal in this order keep
the order you wrote. Each one moves with its `///` comment, its attributes and the comments above
it. Enum cases do not move. Properties declared in the constructor's parameters do not move.

A class with `#[Core\Json\Derive]` writes its JSON keys in the order of its properties. When `order`
changes that order, it prints a note that names the class. With `#[Core\Json\KeepKeyOrder]`, the
properties keep the order you wrote. If no order keeps them, the class is not changed and a
note names it.

`order` prints one line for each changed file, with the number of classes it changed. `--dry-run`
prints one line for each class that would change, and saves nothing. `--check` prints the same lines
and saves nothing. It exits `1` when a class is not in order, so a CI job can run it. A file with a
syntax error is not changed, and standard error gets a line that names it.

```text
$ nvs code order --check
src/Cart.nvs:4:1  Shop\Cart
```

A name is written in full, with its namespace: `Shop\Cart`, `Shop\Cart::addItem`,
`Shop\Cart::$total` or `Shop\Cart::MAX`. Put it in single quotes, so the shell keeps the
backslashes. A position is `path:line:column`, the path relative to the working directory and the
line and column counted from 1, the column in characters. For a project with an interface `Priced`
and a call `$item->price()`:

```text
$ nvs code refs 'Shop\Priced::price'
src/Priced.nvs:5:21    declared  public function price(): int;
public/index.nvs:9:22  use       $total += $item->price();
```

A project with errors still gets a result, from the code the analysis could read. The exit status
is `0`. The number of files that have errors is printed on standard error. A use inside such a file may be
missing from the result. An empty result also exits `0`. A name or a file the project does not
have exits `1` and lists the names that are most similar.

`--json` prints one document with `schemaVersion: 1` and every key present, as `nvs check
--json` does. Each position also has `start` and `end`, counted in bytes from the start of the file.
A failure prints its document too, with `error` and `nearest`, on standard output. Text from the
project is printed with every control character except tab written as `\u{..}`. This is the same in
the text and in the JSON. So the bytes of a file are never sent to the terminal as control codes.

# The compile cache

A compiled unit is stored on disk under a name derived from the hash of its source and of the build
environment — so an entry is never stale and never needs clearing. `nvs run` and `nvs serve` both
read and write it.

`[opcache] file_cache_dir` names the directory, and `[opcache] file_cache = false` turns the cache
off. With no directory named, the cache is the `cache` folder in the data folder. `[cache] dir` is
not a key: it stops the run with `E0601`.

```toml
[opcache]
file_cache_dir = "/var/cache/novis"   # where compiled programs are stored
# file_cache = false                  # compile on every start instead
```

The directory must be owned by the account running `nvs` and writable by no other ordinary account,
and so must the directory containing it. A named directory that fails prints one `warning:` line
and the program is compiled again on every start; [Installing on a host](#tools-install) has the
permissions that pass.

# nvs cache

    nvs cache gc [--json]
    nvs cache clear [--json]

`nvs cache gc` checks the size of the compile cache. If the cache is larger than
`[opcache] file_cache_max_size`, it deletes the oldest entries until the cache is at 80% of that
size. If the cache is not larger, it deletes nothing. The default size is 256 MB.

`nvs cache clear` deletes every entry in the compile cache. The next run of a deleted program
compiles it again.

Both commands read the configuration the way `nvs run` does, so they work on the same cache folder.
They print one line:

```text
removed 3 entries, freed 81920 bytes, skipped 0
```

`--json` prints the same result as one JSON document:

```json
{"schema":1,"removed":3,"freed_bytes":81920,"skipped":0}
```

`skipped` is the number of entries that could not be deleted. On Windows, this is a file that
another program has open. The next `nvs cache gc` deletes it. A file that a run is still writing is
not an entry, so neither command deletes it. If the cache is turned off, both commands print
`removed 0 entries, freed 0 bytes, skipped 0`. Both commands exit with status `0`. They exit with
`1` when the configuration has an error, or when the cache folder cannot be read.
