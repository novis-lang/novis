---
id: cli
title: The nvs command
summary: every subcommand of the `nvs` binary — run, check, test, build, api, config, info, meta, ast — with its flags, its exit status and what it prints
keywords: nvs, nvs run, nvs check, nvs test, nvs build, --compile, --openapi, nvs api diff, nvs config check, nvs config dump, nvs info, nvs meta --json, nvs ast, --config, --dump-ir, --dump-asm, --filter, --format, --php, exit status, exit code, .nvs, .php, shebang, cache, single-file executable, bundle, php -l, php -i, php -r, phpunit, composer, phpdbg
---

# One binary

Everything is one executable, `nvs`. It takes a subcommand, then that subcommand's options, then
usually one file:

    nvs run hello.nvs
    nvs check src/app.nvs
    nvs test tests/
    nvs --version          # prints `nvs 0.0.1 (commit 16a26500b, 2026-09-24)`

| Command | What it does |
|---|---|
| `nvs run <file>` | check, compile and run a program |
| `nvs check <file>` | parse, resolve and type-check, reporting every diagnostic; runs nothing |
| `nvs test <paths>...` | run a program's `#[Test]` methods, or a tree of `.nvst` cases |
| `nvs build --compile <file>` | write a single-file executable holding the program's source |
| `nvs build --openapi <file>` | write the program's OpenAPI 3.1 document to standard output |
| `nvs api diff <old> <new>` | classify every change between two OpenAPI documents |
| `nvs init` | write a `nvs.toml` with every key commented out, into the working directory or to the one path `--config` names — see [Installing on a host](#tools-install) |
| `nvs config check [files]` | resolve the configuration tree and report what it holds |
| `nvs config dump [files]` | print every configuration key in force |
| `nvs info` | build, host and third-party licensing information |
| `nvs meta --json` | the whole `Core` registry as JSON |
| `nvs agent <verb>` | the same registry for a coding agent: a primer, an index, a search, one card — and the pointers `init` installs |
| `nvs ast [--json] <file>` | parse one file and print its syntax tree |

Every subcommand also takes `--config <PATH>` (see `nvs run`) and `-h`/`--help`. `nvs` declares no
short flag of PHP's: there is no `-i`, `-a`, `-r`, `-f` or lowercase `-v`, and every operation is
spelled as a subcommand.

**Not in this build:** `nvs convert` is an unrecognized subcommand. There is no PHP converter in
this binary.

**In other chapters:** `nvs serve`, `nvs ctl` and `nvs service` are in
[the server chapter](#tools-server); `nvs lsp` and `nvs lsp-test` are in
[the editor chapter](#tools-editor).

**Not in this chapter yet:** `nvs queue`, `nvs schema`, `nvs tmp`, `nvs fmt`, `nvs doc` and
`nvs stubs` are in the binary and answer `--help`, but their sections here are unwritten.
`nvs --help` lists every subcommand this build carries.

# Files, extensions and tags

- A program is a `.nvs` file. The extension is a convention, not a rule: the front end never reads
  it, `nvs run` accepts any path, and what matters is the content. `nvs test` is the one command
  that decides anything from an extension, because it has two suites to choose between.
- **Novis does not run PHP.** There is no PHP mode and no PHP front end in this binary: a file
  opening with `<?php` is refused with `E0229` whatever it is named, and renaming a PHP file to
  `.nvs` changes nothing about that. `<?` alone is not a tag at all — the file stays in HTML mode
  and is copied to the output.
- A `#!` first line opens code mode. When the first two bytes of a file are `#!`, Novis skips that
  line and reads the rest as code, as if `<?nvs` stood there, so `#!/usr/bin/env nvs` makes a
  script you run as `./tool` on Linux and macOS. An `<?nvs` in such a file before any `?>` is
  `E0009`. `#!` anywhere else is ordinary text.

```nvs error
<?php
echo "old tag";
```
```output
E0229
```

# nvs run

    nvs run <file> [--config <path>]... [--dump-ir | --dump-asm]

`nvs run` checks the file the way `nvs check` does, stops with the same diagnostics if any, and
otherwise compiles and runs it with the current directory as its working directory. Before running
it reads the configuration:

- `./nvs.toml` in the working directory, if there is one. A missing file is not an error — the
  program runs with an empty configuration.
- `--config <path>` names a file to read *instead*: naming one disables the `./nvs.toml` lookup
  entirely. Repeat the flag to read several files in order; a named file that does not exist
  refuses the run (`E0605`). Relative paths inside a configuration file resolve against that file's
  own directory. The configuration chapter has the file format.

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

`exit("message")` prints the message and exits `0`. An uncaught throwable prints `Uncaught
Exception: <message>` and a backtrace, one `#n Class::method() at file:line` frame per line, to
standard error; nothing about it reaches standard output.

```nvs exit=3
<?nvs
echo "bye", "\n";
exit(3);
```
```output
bye
```

```nvs exit=1
<?nvs
echo "before", "\n";
throw new LogicError("boom");
```
```output
before
```

# nvs check

    nvs check <file> [--autoload-map]

Prints `no errors` and exits `0` when the file and everything it reaches through `require` and
`autoload` type-checks; otherwise prints every diagnostic found and exits `1`. Nothing runs, so it
is the command for an editor, a pre-commit hook or CI. Each diagnostic has one shape:

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
```

A code is `E` followed by four digits for an error and `W` for a warning; the location is
`file:line:column`; the caret marks the span; optional `= help:` and `= note:` lines follow. The
code is stable and is what the tables in this reference cite.

`--autoload-map` prints the resolved `autoload` map instead of `no errors` — every prefix, what a
`discover` glob skipped, what was shadowed, and every root that does not exist — so an autoload
declaration can be audited without running anything. A `{..}` prefix segment is shown as the
directory name it resolved to.

```nvs error
<?nvs
class Counter {
    public int $n = 0;
}
Counter $c = new Counter();
$c->n = "one";
```
```output
E0401
```

# nvs test

    nvs test <paths>... [--filter <text>] [--format human|json|junit] [--php <path>] [--update]

One subcommand runs two kinds of test, and which one is meant is read off the path:

- **A `.nvs` path is a program**, and its `#[Test]` methods are run. Every `public`
  `void` instance method carrying `#[Test]` is one test; classes are reported in name order and
  methods in declaration order; a test that asserts nothing fails; `#[Test(skip: "why")]` skips
  with its reason. The `Core\Test` section has the assertions and the attribute's options.
- **A directory holding `.nvs` files is one program** that requires every one of them, subdirectories
  included, in name order, and its `#[Test]` methods are run. One file in the directory requires the
  application's bootstrap file, which gives the whole directory its `autoload` declarations. A
  directory holding both `.nvs` and `.nvst` files is refused.
- **Anything else is a `.nvst` case file, or a directory walked for `*.nvst`.** A case is one
  program with its expected output, and the report is a conformance summary
  (`3 passed, 0 failed, 0 skipped`). This is the one place an extension decides anything, so a
  program in a file not named `.nvs` is read as a case tree here and fails to parse as one.

The two are not mixed in one invocation. The exit status is non-zero if any test or case failed;
a skipped one is not a failure.

```nvs test
<?nvs
use Core\Test;

final class MathTest {
    #[Test]
    public function addsTwoNumbers(): void {
        Test::assertSame(2 + 2, 4);
    }

    #[Test(skip: "not written yet")]
    public function dividesByZero(): void {
        Test::assertSame(1, 1);
    }
}
```
```output
✓ addsTwoNumbers
- dividesByZero
0 failed, 1 passed, 1 skipped
```

- `--filter <text>` runs only the tests whose name contains the text: a `.nvst` case's path, or a
  program's `Class::method` — so `--filter Class::` selects one class's `#[Test]` methods.
- `--format` chooses how a *program's* run is reported: `human` (the default — one line per test as
  it runs, then `N failed, N passed, N skipped, N flaky in N ms`), `json` (one versioned document
  on standard output at the end: `schemaVersion` is `2`, then `summary`, and one `tests[]` entry per
  test with `class`, `method`, `file`, `line`, `column`, `verdict`, `durationMs`, and
  `reason`/`failures`/`attempts` where they apply),
  or `junit` (JUnit XML). In the machine formats, what the tests themselves `echo` goes to standard
  error so that standard output is the document alone. Naming a machine format beside a `.nvst`
  tree is refused.
- `--list` writes what the program declares without running any of it: under `human`, one
  `Class::method  file:line` per test; under `json`, a `schemaVersion: 2` document whose `listed`
  array holds one `{class, method, file, line, column}` per test and no summary, because nothing
  ran. `--filter` selects the same tests it would select in a run, and `--format junit`, `--update`
  and a `.nvst` tree are each refused beside it.
- `--php <path>` names the PHP binary a case with an `--ORACLE--` section is compared against
  (default `php`).
- `--update` rewrites each failed `Core\Test::assertMatchesInline` snapshot in the source that
  wrote it, and is **the only spelling under which `nvs test` writes to a file at all**. It
  replaces the `$expected` literal and nothing else: a passing snapshot is untouched, and so is
  every other line of the file. The verdicts do not change — the tests that produced a new
  snapshot are still reported as failed, and the re-run is what says the new text is the one you
  meant. Write the snapshot as `''` and let the first run fill it in. Where one method holds two
  snapshots with the same text, neither is written and the run says so: there is no way to tell
  which rendering belongs in which literal. Naming it beside a `.nvst` tree is refused — a case's
  expectation is its `--EXPECT--` section, which nothing rewrites.

A `.nvst` case is a sequence of `--SECTION--` headers: `--TEST--` (one line saying what the case
pins), `--FILE--` (the program, run as `nvs run`), `--EXPECT--` (its exact standard output) or
`--EXPECTF--` (with `%s`/`%d`/`%f` placeholders), `--EXPECTF-ERROR--` (standard error), an
optional `--FILE name--` for each extra file written beside it (`nvs.toml` included), and an
optional `--RUN--` naming another subcommand to run in that directory instead — `test`,
`config dump --origin`.

```text
--TEST--
a read outside the granted roots is refused
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

# nvs build --compile

    nvs build --compile <file> [-o <path>]

Writes a **portable single-file executable**: a copy of the `nvs` binary itself with the program's
source appended — the entry file plus every file its `require` graph reaches, as a flat list of
paths and bytes, followed by a small footer. No compilation result is stored; the bundle compiles
its source when it starts, exactly as `nvs run` would.

- `-o <path>` says where to write it; the default is the entry file's stem in the current directory
  (`routes.exe` on Windows, `routes` elsewhere). The command reports `wrote <path> (N source
  file(s))`.
- Running the bundle runs the program. **Its whole command line belongs to the program**: a bundle
  never interprets `run`, `check`, `--help` or any other `nvs` argument, so an application whose
  first argument happens to be `run` keeps it.
- A bundle still reads `./nvs.toml` from the directory it is *run in*, exactly like `nvs run`, and
  a malformed one refuses the run. Ship the configuration beside it or run it from a directory that
  has none.
- Files reached only through `autoload` at run time, or opened with `Core\IO`, are not in the
  bundle: it carries the static `require` graph and nothing else.

# nvs build --openapi

    nvs build --openapi <file>

Writes an OpenAPI 3.1 document for the program's `#[Route]` methods to standard output: `openapi:
"3.1.0"`, an `info` block whose `title` is the entry file's stem, and one `paths` entry per route
with its method, an `operationId` of `Class::method`, its path parameters with their schemas, and
its responses. Nothing runs. `nvs build` with neither `--compile` nor `--openapi` is refused rather
than doing nothing.

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

    nvs config check [files]...
    nvs config dump [files]... [--origin] [--toml]

Both read the configuration tree offline and run nothing, so a tree can be validated in CI before
it is deployed. With no files named they read `./nvs.toml` (and nothing, successfully, when there
is none); naming files positionally is the same as `--config`.

- `config check` resolves the whole tree — includes, `[[app]]` blocks, secrets — and prints one
  line: `ok: 2 files, 4 directives set, 1 override, 0 warnings`. It exits `1` on anything the tree
  refuses: a TOML syntax error, an unknown key (`E0601`), a missing or cyclic include (`E0605`,
  `E0606`), an `[[app]]` block naming both `root` and `entry` or neither (`E0609`), a secret with
  both `password` and `password_file` (`E0608`). It does **not** check whether a value is a valid
  quantity, or whether a default lies under its own `[limits.hard]` ceiling: `memory = "12
  bananas"` passes `config check` in this build.
- `config dump` prints every key in force, one per line in dotted-key order, with array-of-tables
  blocks numbered (`app.0.root`, `include.1.path`). `--origin` adds the file each key was written
  in, and the file it overrode; `--toml` prints the resolved tree as one canonical TOML document,
  for diffing two environments.

```text
app.0.capabilities.script.spawn = true
app.0.origin                    = "https://example.test"
app.0.root                      = "."
limits.hard.memory              = "512M"
limits.memory                   = "256M"
```

# nvs info

    nvs info [--licenses]

Prints what this binary is: the version, commit, build profile, target triple, Rust and Cranelift
versions under **Build**; the operating system, architecture, CPU parallelism and executable path
under **Host**; and under **Licensing** the `nvs` license (MIT) followed by every third-party
component with its version and the license Novis takes it under. `--licenses` appends every
license text in full. It answers what `php -i` answers, but there is no `nvs -i`: that is an
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

# nvs meta --json

    nvs meta --json [entry.nvs]

Prints the `Core` registry as one JSON object — the same data Part B of this reference is
generated from. `--json` is required. Its seven top-level keys:

- `classes`: one object per `Core` class, with `name` and `members`. A member has `name`, `kind`
  (`static` or `instance`), `signature` (the full spelling, `length(string $s): uint`), `params`
  (each with `name`, `type` and `qualifier` — `neutral`, or the taint or secret admission), `names`,
  `returns`, and `doc` (`short`, a `params` list with a `desc` per parameter, `return`, and `errors`
  where the member throws). A class with constants has a `constants` list.
- `enums`: `name` plus `doc` with `short` and one `cases` entry per case.
- `exceptions`: the throwable tree — `name`, `parent`, and the `properties` a class adds.
- `interfaces`: the global interfaces, with `typeParams` where they have any.
- `attributes`: the compiler-recognized attribute names.
- `directives`: every `nvs.toml` directive with its `key`, its `class` (`Runtime`, `RuntimeTighten`,
  `System`) and when a change applies (`Reload` or `Boot`).
- `capabilities`: one row per gated `Core` member — `class`, `member` and `capability`. It is a
  roster of its own rather than a field on a member, so a renderer that wants a capability beside a
  card joins the two on `(class, member)`; a member absent from it is ungated.

Given an entry point, the command also parses and resolves that program, and adds an eighth key,
`program`, holding its own `classes`, `enums` and `types` (its `type` aliases) in the same shape. A declaration's `doc` is the
`///` comment above it: its prose as `short`, and its `@see` and `@example` tags as lists. The
seven registry keys are byte for byte the same as without the entry point. For a file declaring
one documented class:

```text
$ nvs meta --json cart.nvs
…,"program":{"classes":[{"doc":{"short":"A shopping cart."},"members":[{"doc":{"short":"Adds items to the cart."},"kind":"instance","name":"add","signature":"add(uint $count = 1): uint","visibility":"public"}],"name":"Cart"}]}}
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
scalar fields — an operator, a flag, or which form a member name took — and `children`. A literal's
text is not one of those fields: its span names it, and this command does not type-check, so it
cannot know which literal is `secret` and owes a placeholder rather than its bytes.

The comments and the whitespace the grammar drops are in that document too, as nodes of the same
shape — `Whitespace`, `LineComment`, `BlockComment` or `DocComment`, each with its own span and no
children — sitting under the innermost node whose span contains them, so a comment between two
methods is a child of the class. A consumer wanting only the grammar's own nodes filters on `kind`.

`--resilient` is the default: the tree the parser recovered into is printed whatever it reported,
which is what makes the output useful on the file that does not compile. `--strict` prints no tree
at all once an error is reported. The two are refused together.

# The compile cache

A compiled unit is stored on disk under a name derived from the hash of its source and of the build
environment — so an entry is never stale and never needs clearing. `nvs run` and `nvs serve` both
read and write it.

`[opcache] file_cache_dir` names the directory, and `[opcache] file_cache = false` turns the cache
off. With no directory named it is `novis\opcache` under `%LOCALAPPDATA%` on Windows, and
`novis/opcache` under `$XDG_CACHE_HOME` or `~/.cache` elsewhere. `[cache] dir` is not a key: it is
refused with `E0601`.

```toml
[opcache]
file_cache_dir = "/var/cache/novis"   # where compiled programs are stored
# file_cache = false                  # compile on every start instead
```

The directory must be owned by the account running `nvs` and writable by no other ordinary account,
and so must the directory containing it. A named directory that fails prints one `warning:` line
and the program is compiled again on every start; [Installing on a host](#tools-install) has the
permissions that pass.
