---
milestone: post-parity
position: last
---
# Loop goal 188 — every path a program names starts where it is written, and an attack stops where it says it does

ADR 0241 made a relative path literal at a path parameter start at the folder of the file that wrote
it, and made a relative path built at run time throw. This goal carries that rule to the four places
it did not reach yet, and repairs the attack runner that let it land with attacks that never reached
the step they test. Six items, each one of the user's decisions of 2026-10-01:

```nvs
// 1. A script named in nvs.toml, or pushed onto a queue, starts at the file that names it.
Core\Queue::push('jobs/send-mail.nvs');              // -> <folder of this file>/jobs/send-mail.nvs

// 2. A path typed on a command line can be read.
string $path = Core\Path::fromCwd($typed);           // no `tainted`; throws while a request is answered
echo Core\IO::read($path);

// 3. A class name written as a literal is loaded and checked while compiling.
class<Handler> $h = 'App\Handler\Mail' as class<Handler>;

// 4. The file that wrote the call, and its folder.
string $here = Core\Path::thisFile();
string $data = Core\Path::thisDir('data/rates.json');
```

5. **An attack stops where it says.** A case in `tests/hostile/` that ends early names the step and
   the error class it ends with, and stopping anywhere else fails it.
6. **The perf records are measured last**, once, for every feature this goal and ADR 0241 changed.

## Why here

ADR 0241 landed on `main` as a whole rule with four doors left open, and every one of them is a
place a program meets the working directory again:

- `[[schedule]] script`, `[log] handler` and `Core\Queue::push`'s script all reach
  `crates/nvs-runtime/src/script.rs:@resolve`, which joins a relative path to the working directory.
  The same file therefore names a different script under `nvs serve` and under a service.
  `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` already says every
  path-valued directive starts at its own file, and these two directives are not on its list.
- `Core\Path::fromCwd` is `CoreTy::Text(Qual::Contagious)` (`crates/nvs-stdlib/src/path.rs:196`), so
  its result stays `tainted`, and `Core\IO::read`'s `Qual::Sink` parameter does not accept it. A
  command-line tool cannot read the file its user typed, and
  `docs/examples/core/Path/fromCwd/01-a-file-name-from-the-command-line.nvs` prints the path instead
  of reading it.
- `'X' as class<T>` is left to run time on purpose
  (`crates/nvs-types/src/expr/operators.rs:@reject_impossible_class_reference_conversion`'s doc), so a
  literal names only a class that something else already loaded, and the editor offers only those
  (`crates/nvs-lsp/src/completion.rs:@class_names`).
- A program that wants a file beside itself, and not at a path parameter, has no way to say so.
  `__DIR__` is `E0319`, and ADR 0241 rejected a constant because a forgotten join would still start
  at the working directory. A `Core` member folded while compiling has no such failure.

The attack runner is in the same goal because it is how ADR 0241 slipped. `tools/nv/proofs/run.ts`
accepts any non-zero exit under `// hostile: ends-early`, so several attacks passed by stopping at
the new relative-path error, before the check they were written to test.

It carries `position: last` because it lands behind goal `plain-comments`, whose gate holds every new
comment to `AGENTS.md` § *Text an end user reads*, and it sits in front of goal `foreach-var` and
goal `ci-green` because it still changes the tree they prove.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong, each rewritten whole by the session that lands the
behaviour and not before:

- `docs/rules/programs/path-literals-resolve-from-their-file.md` — its list of path parameters gains
  `Core\Queue::push`'s script, its last paragraph says what `fromCwd` returns, and it names
  `Core\Path::thisFile` and `thisDir` as the way to name the writing file.
- `docs/rules/config/a-relative-path-resolves-against-the-file-it-is-written-in.md` — its list of
  directives gains `[[schedule]] script` and `[log] handler`.
- `docs/rules/security/launderers-are-sink-named.md` — states `fromCwd` as its one exception, and why.
- `docs/rules/types/class-reference.md` — states once that a literal operand of `as class<T>` is
  decided while compiling and a string built at run time sees only loaded classes;
  `docs/rules/programs/no-runtime-autoload.md` points there.
- `docs/rules/testing/hostile-case-contract.md` and `tests/hostile/README.md` § *The directives a
  case may carry* — `ends-early` gains its step and error class.
- `docs/reference/core/Path.md` (its `keywords` gain `__FILE__` and `__DIR__`), the `__DIR__` row of
  `docs/reference/tools/30-php-differences.md`, the queue and schedule chapters under
  `docs/reference/`, and the module docs of `crates/nvs-runtime/src/script.rs`,
  `crates/nvs-server/src/schedule.rs` and `crates/nvs-cli/src/worker.rs` that say a script resolves
  from the working directory.

The search that closes the stage:
`grep -rn "working directory\|ends-early\|fromCwd\|as class<" docs/rules docs/reference docs/examples crates/nvs-runtime/src/script.rs crates/nvs-server/src crates/nvs-cli/src/worker.rs crates/nvs-stdlib/src/path.rs`,
read line by line. Every hit is true as it stands, rewritten, or under `docs/decisions/`.
`docs/novis.md`, `docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated
and are regenerated, never edited.

## Stage 1 — the floor

Goal `plain-comments`'s whole acceptance list, carried in by the goal switch. Never traded. Every
path literal ADR 0241 resolved keeps its answer.

## Stage 2 — an attack stops where it says, the keystone

**Does:** Makes an attack that ends early pass only at the step and error class it declares, and
fixes every attack that stopped earlier.

One file set: `tools/nv/proofs/run.ts`, `tests/hostile/README.md`,
`docs/rules/testing/hostile-case-contract.md`, then the attacks under `tests/hostile/`.

- **The decision record**, one of this goal's five slots, written first. It `modifies`
  `testing/hostile-case-contract`.
- **The directive names its ending.** `// hostile: ends-early <step> <ErrorClass>`: the numbered
  step the program stops in, and the class of the error it stops with. A fatal limit has a class the
  runtime prints, and that is the name written. A marker with no step or no class fails the case.
- **The runner reads where it stopped.** `run.ts:@judgeHostile` finds the step the program stopped
  in and the error class it stopped with, and fails the case when either is not the declared one.
  The message names both steps or both classes: `stopped at step 2 with RuntimeError, declares step 4
  with MemoryLimitError`. How the step is found is the session's call: the line the error report
  names, mapped to the numbered step comment above it, needs nothing from the attack; a line each
  step prints before it starts is the fallback when a report names no line. Not checked which one
  every fatal kind allows.
- **Every attack is migrated, and none is softened.** About 240 files declare `ends-early` today.
  Each gains its step and class from a run of it. An attack that stops before its last step is fixed
  so that the step it was written to test runs: a relative path made absolute with
  `Core\Path::join`, a throw it did not mean to reach caught. It is never fixed by moving the marker
  to the step it stops in. `tests/hostile/README.md` § *When a case actually breaks something* is
  the rule for an attack that now finds a real failure.
- **Pinned by** `tools/nv/test/hostile-ending.test.ts`: the four verdicts of the check, and one test
  that reads every attack under `tests/hostile/` and fails on an `ends-early` without a step and a
  class.

## Stage 3 — scripts start at their file

**Does:** Resolves `[[schedule]] script` and `[log] handler` against the folder of their `nvs.toml`,
and makes `Core\Queue::push`'s script a path parameter that stores an absolute path.

Two file sets, in this order. The configuration: `crates/nvs-config/src/tree.rs` (the `handler`
field at line 477 and the schedule's `script` at line 1055), `crates/nvs-config/src/resolve.rs`,
`crates/nvs-server/src/schedule.rs`, `crates/nvs-host/src/ladder.rs:102`,
`crates/nvs-cli/src/serve.rs:1741`. The queue: `crates/nvs-stdlib/src/queue.rs` (the `push`
descriptor at line 1908), `crates/nvs-cli/src/worker.rs:1379`, `crates/nvs-types/src/paths.rs`.

- **The decision record**, one slot. It `modifies`
  `config/a-relative-path-resolves-against-the-file-it-is-written-in` and
  `programs/path-literals-resolve-from-their-file`, and `rule:concurrency/queue-four-members` if its
  sentence on `push` names how the script is found.
- **The two directives** resolve while the configuration is read, the way every other path-valued
  directive does: the boot log and `nvs config dump` print the absolute path, and a directive in an
  `[[include]]`d file starts at that file.
- **`Core\Queue::push`'s script** is marked as a path in the registry, `CoreTy::Path(Qual::Sink)`
  in place of `CoreTy::Text(Qual::Sink)`, so a `tainted` script is still refused and a relative literal there
  starts at the file that wrote the call (`crates/nvs-types/src/paths.rs:@resolve_args`), and a
  relative path built at run time throws at the push with the error `relative_refusal` gives
  (`crates/nvs-runtime/src/capability.rs:127`). The row a job is stored in holds the absolute path,
  so a worker started in any folder runs the same script.
- **`script::resolve` reads no working directory** for any of these four. A relative path reaching
  it is a defect upstream, and it throws.
- **The tests that broke are migrated.** `crates/nvs-cli/tests/live_edit.rs` and
  `crates/nvs-host/tests/limits.rs` wrote their scripts relative to the working directory. They now
  write them beside their configuration, or name them absolute. A test is never kept passing by
  reading the working directory for it.
- **A job already in a queue** with a relative script is not rewritten. It fails when a worker takes
  it, with the same error, and the record says so.
- **Pinned by** the Stage 3 checks. The `nvs-cli` pair starts the program in a working directory that
  is not the configuration's folder.

## Stage 4 — a path typed on the command line

**Does:** Makes `Core\Path::fromCwd` return a plain string, so a command-line tool reads the file its
user typed.

One file set: `crates/nvs-stdlib/src/path.rs` (the descriptor at line 196, the body at line 1125),
`docs/rules/security/launderers-are-sink-named.md`, `docs/examples/core/Path/fromCwd/`,
`tests/hostile/core/Path/fromCwd/`.

- **The decision record**, one slot. It `modifies` `security/launderers-are-sink-named`, which
  states this as its one exception: `fromCwd` throws while a request is answered, so request data
  never passes through it, and the `fs` grants still bound every file it can name. A request keeps
  `Core\IO::within($base, $path)`.
- **The return type** is a plain `string`. Whether that is a new `CoreTy` row or a `Qual` the
  registry already has is the session's call; what is fixed is that `secret` is not removed.
- **Example 01** reads the file its user typed and prints its first line. It ships the file it reads
  beside it, and how the example runner's working directory reaches that file is checked before the
  example is written. Not checked.
- **The attack** under `tests/hostile/core/Path/fromCwd/` gains a step: a request that passes its own
  data to `fromCwd` throws, so that data never loses `tainted` there.
- **Pinned by** the Stage 4 checks.

## Stage 5 — `thisFile` and `thisDir`

**Does:** Adds `Core\Path::thisFile()` and `Core\Path::thisDir()`, folded while compiling to the path of
the file that wrote the call and its folder.

One file set: `crates/nvs-stdlib/src/path.rs`, `crates/nvs-types/src/paths.rs`,
`crates/nvs-types/src/expr_table.rs`, `crates/nvs-ir/src/lower/expr.rs`, then the proofs.

- **The decision record**, one slot. It names ADR 0241's rejected `__DIR__` constant and says why a
  `Core` member folded while compiling is not that alternative: it is written only where a program
  wants the folder, and a path parameter still needs no join. It `modifies`
  `programs/path-literals-resolve-from-their-file`.
- **No name clash.** `Core\Path` has no member of either name today (`crates/nvs-stdlib/src/path.rs`'s
  descriptor table, checked when this goal was written).
- **`thisFile(): string`** is the absolute path of the file that contains the call, and
  **`thisDir(?string $join = null): string`** is its folder, joined with `$join` when one is given.
  The base is `crates/nvs-types/src/paths.rs:@base_folder`'s, the one a path literal joins to, so a
  bundle answers the folder beside the executable.
- **Folded while compiling.** The call is recorded the way `resolve_literal` records a path literal
  (`ExprTypeTable::path_literal`), and `nvs-ir` lowers it to a string constant. It costs nothing at
  run time and calls nothing.
- **`$join` is a relative string literal**, joined lexically as a path literal is. A `$join` that is
  a variable, a constant or an absolute literal does not compile, with one new diagnostic code
  (check the next free one) whose text says to write `Core\Path::join(Core\Path::thisDir(), $part)`.
- **Magic constants stay `E0319`.** Its card text and the `__DIR__` row of
  `docs/reference/tools/30-php-differences.md` name the two members.
- **Feature proofs for both members**, by `rule:testing/feature-proofs`: an `about.md`, tests from
  Novis and from Rust, three examples, one bench, one attack and a reference card each
  (`rule:core-api/reference-card`). `bun nv proofs --id 'Core\Path::thisFile'` prints what is
  still owed. A conformance case cannot freeze an absolute path, so it prints what is the same on
  every machine: the base name, `isAbsolute`, and that `thisDir()` is `dirname(thisFile())`.
- **Pinned by** the Stage 5 checks.

## Stage 6 — a class name written as a literal

**Does:** Makes `'X' as class<T>` load and check its class while compiling, as `X::class as
class<T>` does, and offers every class that can be loaded in the editor.

Two file sets, in this order. The compiler: `crates/nvs-types/src/expr/operators.rs`,
`crates/nvs-hir/src/requires.rs`, `crates/nvs-hir/src/autoload.rs`. The editor:
`crates/nvs-lsp/src/completion.rs`.

- **The decision record**, one slot. It `modifies` `types/class-reference` and states the rule once
  there; `programs/no-runtime-autoload` points to it and keeps its own sentence.
- **A plain string literal** under `as class<T>` and `as ?class<T>` is read as the class's whole
  name. The class is loaded through `autoload` while compiling, as `X::class` loads it, and the
  conversion is checked: a name no declaration or `autoload` entry answers does not compile, and a
  class that is not a `T` does not compile. Each reuses the code `X::class as class<T>` already
  gives for the same mistake.
- **Everything else is unchanged.** A class constant, a concatenation and a variable are values
  built at run time and still see only loaded classes.
- **Completion** in that string offers every class `autoload` can load that is a `T`, not only the
  loaded ones. `class_names`'s doc says what it offers now.
- **Pinned by** the Stage 6 checks. The feature whose proofs grow is the one whose reference
  heading covers `as class<T>`; `bun nv brief --where class-reference` finds it.

## Stage 7 — the perf records

**Does:** Measures the perf figure of every feature this goal and ADR 0241 changed, once, after the
code has stopped changing.

No crate is open. `bun nv proofs --record-perf --only <feature>` once for each of
`Core\Path::fromCwd`, `lang:programs/file-paths-a-literal-starts-at-the-folder-of-its-file`,
`Core\Path::thisFile` and `Core\Path::thisDir`, and for any other feature `bun nv proofs --gate` then
names as stale. The figures are not recorded in an earlier stage: every stage before this one moves
code they measure.

## Standing decisions

- **The user's six calls**, written above as the items, are not re-decided. Where they leave a
  question, these are the goal writer's calls and the user has not confirmed them: the directive's
  shape in Stage 2, the queue rows already stored in Stage 3, `$join` as a literal only in Stage 5,
  and reused codes in Stage 6.
- **Five ADR slots**, one each for Stages 2 to 6, and no other number. Each is checked against
  `docs/decisions/` right before it is written, because another agent may take a number first. Each
  states its tradeoffs. Performance: nothing on the request path; Stages 3 and 5 move work from
  run time to compile time. Memory: nothing. Usability: a script, a typed path and a file beside the
  program work the same from every folder. Simplicity: two members and one exception to a security
  rule more, and one place fewer that reads the working directory.
- **The working directory is read in two places only**: `fromCwd`, and the name typed after
  `nvs run`. A session that finds a third puts it in the handoff's `## Backlog`.
- **`fromCwd` removes `tainted` and nothing else.** No other launderer is added or widened.
- **An attack is never softened to pass** (`tests/hostile/README.md`). An attack that finds a real
  failure is fixed or recorded with a gap.
- **Every name in a test, an example and a record is neutral** — `Blog`, `Shop`, `Mail`.
- **Every comment in a new `.nvs` and every changed `about.md` follows `AGENTS.md` § *Text an end
  user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over them before
  the wrap.
