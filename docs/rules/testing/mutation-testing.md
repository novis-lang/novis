A mutation run makes a small, deliberate change to the code, re-runs the tests, and records whether
anything failed. It is the measurement that closes the gap coverage leaves: a line having run says
nothing about whether an assertion would have objected had it been wrong, and a suite at full line
coverage routinely misses most introduced defects.

Two things make the built-in version categorically better than an external one, and neither is
available outside the compiler. Mutants are generated from typed IR, so every one is type-valid and
compiles, where a tool mutating a syntax tree pays a compile to discover that many do not. And
mutant runs are **coverage-directed**: the probe data says which tests touch the mutated line, so a
mutant runs against those rather than against the whole suite. On two thousand tests and five
hundred mutants that is the difference between minutes and most of a day.

**A mutant is one changed site in the lowered program, and a run compiles the program once.** Parse
and check run once. Lowering then arms every mutant whose tests run: each armed site lowers its
original and its changed form behind one test of the run's *active mutant number*, and codegen
compiles that program once. The number is one word the compiled unit owns, read with one load, and the
run sets it before each mutant's tests, so only that mutant's site takes its changed form. It is one
compiler with a switch, like `rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`: only a
`--mutate` run lowers it, a unit with it is never written to the artifact cache, and a normal build and
a server pay nothing. No mutant is made from test code (`rule:testing/tests-never-reach-a-build`'s
marker methods, and every body written inside one), so a mutant changes the program under test and
never the test that judges it.

**Every mutant names its function and has a key.** The function is the label of the method or
property hook the site is written in, `Shop\Cart::total`. A site in an anonymous function or a
generator body names the method it is written in, so a user finds it by the name they wrote. The key
is `{function}#{n}: {change}`, such as ``Shop\Cart::total#2: `<` to `<=` ``, where `n` counts that
function's mutants with the same change, in source order, from 1. An edit outside the function does
not change a key, and the line number is not part of it. A rename of the function does. Keys are
numbered over the whole program before a run is aimed, so aiming never changes one.

**A run is aimed.** `--mutate` alone mutates the whole program. `--mutate=<target>` takes one or more
comma-separated targets, and the `=` is required, so a path after `--mutate` is never read as one:

| Target | Mutates |
|---|---|
| a folder or a file, relative to the current folder | every site in the program's files under it |
| `Shop\Cart` | every site in that class's methods |
| `Shop\Cart::total` | every site in that method |

A target that exists as a path is a path. A target is judged once the program is checked, before any
test runs, and one that matches no file, class or method of the program exits with status 2 and names
it. The filter is applied to the survey, so an aimed run compiles only the mutants it keeps.

**`#[NoMutate(because: "…")]` keeps code out.** It is a `Core\Test` attribute on a class or a method.
`because:` is required and may not be empty, as `skip:` may not (`E0880`). A site under it makes no
mutant, the way test code makes none. It has no effect outside `--mutate`, and a normal build pays
nothing for it. There is no opposite attribute: `--mutate=<target>` aims a run without editing the
code, and one attribute is simpler than two.

**`--since <ref>` mutates only what changed.** With `--mutate`, it keeps a mutant when any line of its
span was added or edited in the working tree since the git ref, or is in a file git does not track.
With a target as well, both must hold. A line that was only deleted leaves no line to mutate. Every
test still runs under `--mutate`, because a mutant runs the tests coverage saw reach it. Without
`--mutate`, `--since` selects tests instead (`rule:testing/tags-and-shards-select-what-runs`). `nvs`
runs the `git` program in the program's folder (the folder `nvs.toml` is in, or the entry file's),
as a child process with no shell, and gains no git library:

| Command | Gives |
|---|---|
| `git rev-parse --show-toplevel` | the checkout's root, which every path git prints is relative to |
| `git rev-parse --verify --quiet --end-of-options <ref>^{commit}` | whether git knows the ref |
| `git diff --no-ext-diff --no-color --no-textconv --no-relative --unified=0 --src-prefix=a/ --dst-prefix=b/ --end-of-options <ref> --` | the changed lines of each tracked file |
| `git ls-files -z --others --exclude-standard --full-name` | the untracked files, every line changed |

The prefixes and `--no-relative` are spelled out so a user's `diff.noprefix` or `diff.relative`
setting cannot move a path. A ref that starts with `-` is refused when the arguments are read, before
git runs: no branch, tag or commit is named that way, and `--since=--output=x` would reach git as an
option that writes a file. `--end-of-options` keeps every other ref a ref as well. Three errors exit
with status 2 before any test runs, and each says which it is: no `git` on `PATH`, a folder outside a
checkout, and a ref git does not know. `--since` never falls back to mutating everything. Beside a
`.nvst` tree it exits with an error, as `--mutate` does. It runs only on a developer's machine or in
CI, and it spends one list of line ranges per changed file for the run.

The first operator set is six swaps, each keeping its operands' and its result's types, so every
mutant lowers and compiles with no second check:

| Operator | Change |
|---|---|
| relational boundary | `<` ↔ `<=`, `>` ↔ `>=` |
| equality negated | `==` ↔ `!=` |
| arithmetic swap | `+` ↔ `-`, `*` ↔ `/` |
| logical connector | `&&` ↔ `\|\|` |
| boolean literal | `true` ↔ `false` |
| branch condition negated | an `if`'s or a loop's condition is inverted |

A swap is made only where it keeps the result type, so `*` and `/` swap over `float` and `decimal`
alone. A condition whose narrowing changes how a value is read keeps its direction, its connectors
and that test: an `is` test, and an `==` or `!=` that compares a value stored tagged (`?T`, a union,
`mixed`) with a literal, `null` or a class constant. A mutant that took such an edge with the test
false would read a value at a representation it does not have. An equality over a value stored any
other way narrows it to a single value or an enum case, which is stored as its base, so
`int $n == 10` and `$mode == Mode::Off` are reversed and their branches negated. A variable is judged
by its slot, which an earlier test does not untag, so the line is drawn on the safe side. A `true` or
`false` that is a condition, a `match` subject or an `==` operand keeps its value.

The set is enumerated where it is implemented, so adding an operator changes no rule.

**Glue makes no mutant.** A site whose nearest enclosing call is a `Core\Log` method, or that is in
the arguments of the `new` a `throw` throws, changes a message and never a result. Google's mutation
work calls these *arid nodes*. A call nested inside a glue call decides for its own arguments, so
`Log::write(..., ['n' => $cart->total($a + 1)])` still mutates `$a + 1`. Output written with `echo`
or `Core\Html` is not glue: in a web program the page is the result, and a view class is kept out
with `#[NoMutate]`. The list is enumerated where it is implemented, beside the operators
(`nvs_ir::lower::mutate`), so adding a case changes no rule.

`--mutate` runs on a program's `#[Test]` methods only. On a `.nvst` tree it exits with an error, as
the coverage flags do, because per-test coverage exists only for program tests.

**The run.** `nvs test --mutate` runs the selected tests once with coverage, as a coverage flag does
(`rule:testing/coverage-report`), and reports that run as any run is reported. A test that fails there
fails the run, and no mutant runs: a mutant is judged against a suite that passes without it. Then
each mutant runs the selected tests that reached the statement it changes, the innermost statement
whose span holds the changed expression, up to `--jobs` at once, and no further test starts once one
has failed. A test with no wall-time limit gets one for a mutant run, ten times the slowest test of
the coverage run plus one second, because a mutant can make a loop that never ends; a test with its
own limit keeps it (`rule:testing/a-test-has-a-wall-time-limit`).

**A mutant that compiles to code already judged runs no test.** This is Trivial Compiler
Equivalence. Before a reached mutant's tests run, the function its change is in is compiled alone,
at the run's optimisation level, and hashed: its machine code, each relocation by its target's name,
and each data object it defines. The function is the compiled one the change landed in, so a change
inside an anonymous function hashes that function. Coverage probes are numbered from zero in this
compile, so the hash does not depend on where the function sits in the program. The program's own
function is hashed once, the first time a mutant changes it. The run keeps one hash per changed
function and per mutant.

| Outcome | When |
|---|---|
| `equivalent` | the changed function compiles to the program's own code |
| `duplicate` | it compiles to an earlier mutant's code in the same function; the record's `sameAs` is that mutant's key |

Cranelift folds far less than an optimising C compiler, so the share is small. A `true` stored in a
variable that is never read is `equivalent`. An `==` negated and the `if` around it negated compile
alike, so the second is a `duplicate`. `$x + 0` and `$x - 0` do not compile alike, because each
operation checks for overflow. On `benches/scaling/testing/mutate.nvs`'s program, none of the 12
mutants was `equivalent` or `duplicate`.

Each mutant has one of six outcomes:

| Outcome | When |
|---|---|
| `killed` | a test failed or called `exit` |
| `timed out` | no test failed, and a test ran past its wall-time limit |
| `survived` | every test it ran passed, a flaky one included |
| `not reached` | no selected test reached its statement, so nothing ran |
| `equivalent` | its function compiles to the program's own code, so nothing ran |
| `duplicate` | its function compiles to an earlier mutant's code, so nothing ran |

**`mutants.toml` holds the judged survivors.** It is an optional file in the program's folder (the
folder `nvs.toml` is in, or with no project the entry file's), read only under `--mutate`:

```toml
[[judged]]
mutant = "Shop\\Cart::total#2: `<` to `<=`"
reason = "the loop ends on the same item either way"
equivalent = true
```

`mutant` and a non-empty `reason` are required, `equivalent` defaults to `false`, and no other key is
allowed. A malformed file, a missing or empty `reason` and a key judged twice exit with status 2
before any test runs. A survivor whose key has an entry is *judged*: the human report counts it and
does not list it. A judged survivor that is `equivalent` leaves the score, like an `equivalent`
mutant; any other still counts as survived, so the score stays a measurement. An entry whose key no
mutant of the program has is printed as a warning on stderr and changes no exit status, so the file
does not rot. The key is checked against every mutant of the program, not only the ones an aimed
run kept, so `--mutate=<target>` and `--since` warn about nothing they left out. No command writes
the file: each entry is a person's judgment, and the human report prints each survivor's key as a
line ready to paste.

The score is killed plus timed out over every mutant that ran a test, less the judged `equivalent`
survivors. The human report prints the four counts of mutants that were judged by tests, then the
`equivalent` and `duplicate` counts when either is not zero, then the judged count when it is not
zero, then the score, then the survivors no entry judged, **grouped by function**. Functions are
ordered by their number of survivors, most first, then by name, under a `survived in <function>
(<n>):` line. Within a function each source line is one line: its file and line, the first
survivor's change, `(+N more on this line)` when there are more, and `mutant = "<key>"`, the line to
paste into `mutants.toml`. A `duplicate` of an unjudged survivor is listed with it and counted in
`+N`, because it would have come out the same, and its line names the first mutant's key.
`--format json` writes `schemaVersion: 5` (`rule:testing/report-formats`): version 3's document, plus
a `mutants` array with one record per mutant, `{file, line, function, key, change, outcome, tests}`,
`sameAs` on a `duplicate`, and `judged: true` with its `reason` and `equivalent` on a judged
survivor, in the order the program lowers its sites. `tests` is how many tests
reached the mutant's line and ran against it, `0` for a mutant that ran none. It is what the
covering-test filter decides and what the bench of a mutation run counts. A JUnit document has no place for a mutant, so `--mutate` with
`--format junit` exits with an error. The exit status is the coverage run's: a survived mutant does not
fail the run.

**What it costs:** nothing for a run without `--mutate`, and nothing on the request path. A mutation
run pays one coverage run, then one compile of each reached mutant's changed function to judge it.
The judge lowers the program in rounds, each with at most one mutant of every function fixed on, so
it lowers and declares the program once per round, plus once unchanged, and never once per mutant:
the rounds are as many as the most mutants one function has. The run then lowers and compiles the
program once with every mutant whose compile matched no earlier code, and runs each one's tests that
reach its line. That unit is bigger than the program's by one changed form and one load and branch
per armed site, and a test under it runs that much slower. Memory is O(`--jobs`): one mutant runs at
a time, with its tests in flight, and the run keeps one record and one code hash per mutant for the
report, its function label and key included. One lowered program is held at a time while judging. A
compile records one label per `#[NoMutate]` class or method. `mutants.toml` is held as one key and
one reason per entry for the run, and the run keeps every mutant's key to check the entries against.

A run's time grows linearly with the program. Two ladders measure it.
`benches/scaling/testing/mutate-program.nvs` grows the program, n price rules with their own mutants
and a one-test class each, and its clock grows with a slope of 1.13 from 16 to 128 rules. At 512
rules, 1,536 mutants, a run took 1.5 seconds. With a whole-program lowering and declaration per
mutant it took 10.9 seconds and grew with the square. `benches/scaling/testing/mutate.nvs` grows the
suite around a fixed program, and its ramp fails when the test runs per mutant grow with the suite,
which is what the covering-test filter decides.

**Declined:**

- *Keeping every equality against a literal, `null` or a constant*, which the syntax alone decides.
  Nearly every condition an application writes is one, so the equality and branch operators made no
  mutant in most programs.
- *Mutating the syntax tree.* Each mutant would pay a check to find out whether it compiles, and the
  tree has no types for a swap to keep.
- *Recompiling only the mutated function.* It needs a partial-recompile mechanism kept correct beside
  the whole-program one, for a cost the per-mutant test run already outweighs.
- *Running several mutants at once.* Each would hold a compiled unit and its tests' isolates, and
  `--jobs` already fills the cores with one mutant's tests.
- *A key with the line number in it.* An edit above the function would change every key below it.
- *`#[Mutate]` as an opt-in.* `--mutate=<target>` aims a run without editing the code.
- *Globs in a target.* A folder, a class and a method name every set a run has needed.
- *A per-test limit from each test's own coverage-run time.* It is closer for a fast test, and it
  needs a table carried into every test's run; one bound for the run is one number.
- *A command that writes `mutants.toml`.* Each entry is a judgment, and a command would accept a
  survivor nobody read.
- *Output as glue.* `echo` and `Core\Html` write the page, which is a web program's result.
