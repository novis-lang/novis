---
id: testing
title: Testing
summary: `#[Test]` methods, `nvs test`, a directory of test files as one program, fixtures, data rows, the assertion roster, fixed clocks and seeds, and the report formats
keywords: test, #[Test], nvs test, test directory, test suite, bootstrap, PHPUnit, TestCase, assertEquals, assertSame, assertTrue, assertNull, assertCount, expectException, assertThrows, dataProvider, TestWith, Fixture, setUp, skip, markTestSkipped, retries, flaky, Core\Test, Core\Test\Failure, --format json, junit, fixed clock, seed, .nvst, how to test, unit test, which test, expected output, phpt
---

# A test is a method marked `#[Test]`

**To test your own code, write `#[Test]` methods.** `nvs test` also runs `.nvst` case files, which
compare the text a whole program prints with an expected text. *Which kind of test to write*, under
*Running tests* below, says when a `.nvst` case is the right one.

Testing is part of the language: `#[Test]` on a method marks it, the compiler collects every
`#[Test]` in the program into a table while checking it, and `nvs test file.nvs` runs that table.
There is no base class to extend and no naming convention — any class in the program may hold
tests. `nvs test tests/` runs every `.nvs` file under a directory as one program.

```nvs test exit=1
<?nvs
use Core\Test;

final class ArithmeticTest {
    #[Test]
    public function additionCommutes(): void {
        Test::assertSame(2 + 3, 3 + 2);
    }

    #[Test(skip: "division is not written yet")]
    public function divisionRounds(): void {
        Test::assertSame(7 / 2, 4.0);
    }

    #[Test]
    public function thisOneFails(): void {
        Test::assertSame(2 + 2, 5);
    }
}

echo "the entry file's statements do not run under nvs test", "\n";
```
```output
  ArithmeticTest
      skipped: division is not written yet
      Core\Test::assertSame failed: `$actual` is 4, `$expected` is 5
  1 failed, 1 passed, 1 skipped, 0 flaky in
```

The human report prints one line per test as it runs — `✓` passed, `✗` failed with the failure's
message under it, `-` skipped with its reason, `!` flaky — grouped by class, and a summary line.
Each test line ends with its duration. `nvs test` exits `1` when any test failed and `0`
otherwise; a skipped test is not a failure.

- `use Core\Test;` makes the marker `#[Test]` and the assertions `Test::assertSame(…)` one short
  name; `#[Core\Test]` written out in full is the same attribute.
- A `#[Test]` method is `public`, an instance method, returns `void`, and its parameters are filled
  by fixtures or data rows (below) — a `static`, non-`public` or value-returning `#[Test]` is a
  compile error, and so are two `#[Test]` methods of one class sharing a name.
- Classes are reported in name order; within a class, tests run in declaration order.
- The entry file's top-level statements do **not** run under `nvs test`; every file the
  `require`/`autoload` graph reaches is still compiled, so the classes under test are declared.
- `nvs test <directory>` runs every `.nvs` file under the directory as one program. *A directory
  of test files is one program*, under *Running tests* below, says how.
- A test that asserts nothing fails — write `Core\Test::assertDoesNotThrow` when the claim is that
  a call completes.

# Every test is its own isolate, and the constructor is `setUp`

Each test runs in a fresh isolate over the same compiled program: a new instance of its class is
constructed for it, and a `static` one test wrote reads its declared initial value in the next.
The class's constructor is therefore PHPUnit's `setUp`, and it must take no arguments — a
constructor with parameters is reported as that test failing.

```nvs test
<?nvs
use Core\Test;

final class Counter {
    public static int $hits = 0;
}

final class IsolationTest {
    private array<string> $log;

    public function constructor() {
        $this->log = ["constructed"];
    }

    #[Test]
    public function firstWrites(): void {
        Counter::$hits = Counter::$hits + 1;
        $this->log[] = "first";
        Test::assertSame(Counter::$hits, 1);
        Test::assertCount($this->log, 2);
    }

    #[Test]
    public function secondStartsFresh(): void {
        Test::assertSame(Counter::$hits, 0);
        Test::assertCount($this->log, 1);
    }
}
```
```output
  IsolationTest
  0 failed, 2 passed, 0 skipped, 0 flaky in
```

A test that leaves a task running when its body returns fails, named as such; `Core\Task::all`
and `::map` return with nothing running, so only an unawaited `spawn script` reaches that.

# `#[Test(...)]` options

Every option is optional, and an unknown or mistyped one is a compile error:

| Option | Type | Effect |
|---|---|---|
| `skip:` | `string` | The test is reported skipped with this reason and never constructed; its fixtures are not built. `skip: true` is refused — a skip states a reason. |
| `at:` | `string` | Fixes the test's clock at this instant (`"2026-01-01T00:00:00Z"`): `Core\Time::now()` reads it, and `Core\Test::advance` moves it. |
| `seed:` | `int` | Seeds `Core\Random` and `Core\Uuid::v7` for this test, so two tests with one seed draw one sequence. Without it they draw from the CSPRNG. |
| `retries:` | `int` | Attempts allowed after a failure. Needs `because:` beside it — `retries:` alone is refused. A test that fails and then passes is reported **flaky**, never green, with the failed attempt's message; one that fails every attempt is a plain failure carrying the last attempt's message. |
| `because:` | `string` | The written reason for `retries:`. Allowed on its own. |
| `db:`, `server:` | `string`, `bool` | Accepted by the checker; nothing reads them in this build. |

```nvs test
<?nvs
use Core\Test;
use Core\Time;
use Core\Random;

final class DeterminismTest {
    #[Test(at: "2026-01-01T00:00:00Z")]
    public function theClockIsFixedAndMovable(): void {
        Test::assertSame(Time::now()->toIso(), "2026-01-01T00:00:00Z");
        Test::advance(90m);
        Test::assertSame(Time::now()->toIso(), "2026-01-01T01:30:00Z");
    }

    #[Test(seed: 42)]
    public function aSeedIsRepeatable(): void {
        int $first = Random::int(0, 999);
        Test::assertTrue($first >= 0);
    }

    #[Test(retries: 2, because: "a counter stands in for a flaky network")]
    public function itPassesOnTheSecondAttempt(): void {
        Attempts::$n = Attempts::$n + 1;
        Test::assertSame(Attempts::$n, 2);
    }
}

final class Attempts {
    public static int $n = 0;
}
```
```output
  DeterminismTest
      flaky: it passed on attempt 2, having failed with:
  0 failed, 2 passed, 0 skipped, 1 flaky in
```

`Core\Test::advance` outside a test with `at:` throws a `LogicError`; the host clock is never
moved. The retry allowance is spent inside the test's isolate, so a static counter survives across
attempts, as above.

```nvs error
<?nvs
use Core\Test;

final class ShapeTest {
    #[Test]
    public static function aTestIsNotStatic(): void {}

    #[Test(retries: 2)]
    public function aRetryStatesItsReason(): void {}
}
```
```output
the `#[Test]` method `aTestIsNotStatic` is `static`
```

# `#[Core\Test\Fixture]`: built once, injected by type

A `#[Fixture]` is a `public static` method of the test class whose **return type** is what it
supplies: every `#[Test]` (or other fixture) of that class declaring a parameter of that type
receives it. A fixture is built once per class, in the runner's own context, and a copy crosses
into each test's isolate — the expensive setup runs once, and no test can see another's writes.

```nvs test
<?nvs
use Core\Test;
use Core\Test\Fixture;

final class Schema {
    public function constructor(public string $name) {}
}

final class Repo {
    public function constructor(public string $tag) {}
}

final class RepoTest {
    #[Fixture]
    public static function schema(): Schema {
        return new Schema("users");
    }

    #[Fixture]
    public static function repo(Schema $schema): Repo {
        return new Repo("repo of " . $schema->name);
    }

    #[Test]
    public function itIsHandedWhatItsTypeNames(Schema $schema): void {
        Test::assertSame($schema->name, "users");
    }

    #[Test]
    public function fixturesChain(Repo $repo, Schema $schema): void {
        Test::assertSame($repo->tag, "repo of users");
    }
}
```
```output
  RepoTest
  0 failed, 2 passed, 0 skipped, 0 flaky in
```

- Resolution is by type and by class: a parameter's name is irrelevant, one class supplies each
  type at most once, and another class's fixtures are not in scope. A parameter nothing supplies,
  a cycle between fixtures, and a fixture that is not `public static` or returns `void` are
  compile errors. `#[Fixture]` takes no payload.
- A fixture is built only if a test that runs asks for it; a skipped test's fixture is not built.
- A variadic or `inout` parameter on a test or fixture is refused.

# `#[Core\Test\TestWith]`: data rows

`#[TestWith(param: value, …)]` supplies one row of arguments, matched to the method's parameters
by name and by type; repeat it for each row. Each row is its own reported case, `method#0`,
`method#1`, … in written order. It replaces PHPUnit's `@dataProvider`.

```nvs test exit=1
<?nvs
use Core\Test;
use Core\Test\TestWith;

final class Slug {
    public static function of(string $title): string {
        return Core\Str::lower(Core\Str::replace(Core\Str::trim($title), " ", "-"));
    }
}

final class SlugTest {
    #[TestWith(title: "Hello World", want: "hello-world")]
    #[TestWith(title: "  padded ", want: "padded")]
    #[TestWith(title: "Mixed Case", want: "mixed case")]
    #[Test]
    public function itSlugs(string $title, string $want): void {
        Test::assertSame(Slug::of($title), $want);
    }
}
```
```output
  SlugTest
    ✓ itSlugs#0
    ✓ itSlugs#1
    ✗ itSlugs#2
      Core\Test::assertSame failed: `$actual` is "mixed-case", `$expected` is "mixed case"
  1 failed, 2 passed, 0 skipped, 0 flaky in
```

- A row value is a constant of the parameter's declared type (`int`, `uint`, `float`, `bool`,
  `string`); a field naming no parameter, a mistyped one, one given twice, and a row that omits a
  parameter another row fills are compile errors.
- Rows and fixtures mix in one method: a parameter no row names is filled by type from the class's
  fixtures; where both could answer, the row wins.
- `#[TestWith]` on a method that is not a `#[Test]` is refused. `skip:` on a test with rows skips
  every row.

# Assertions

Every assertion is a static method of `Core\Test`, subject first (`$actual`, then `$expected`),
with an optional trailing `{message: "…"}` written in front of the failure's own report. A
passing assertion returns and throws nothing; a failing one records the failure in the test's
ledger and throws `Core\Test\Failure`. The verdict is read off the ledger, so **catching a
`Failure` does not make the test pass** — a test that swallowed its own failure still fails, and
every failed assertion of a test is reported, not only the first.

| Member | Holds when |
|---|---|
| `assertSame($a, $e)` | identical — scalars and arrays by value, two objects only when they are the same object (PHPUnit's `assertSame`) |
| `assertEquals($a, $e)` | as `assertSame`, except two objects compare through `Comparable::compareTo`; an object without one throws a `RuntimeError` |
| `assertEqualsDeep($a, $e)` | structurally equal — arrays entry by entry, objects property by property — and the failure names the path of the first difference (`$actual->rows["1"]`); a `secret` property is compared but shown redacted |
| `assertTrue($a)` | the `bool` is `true`; the argument is a declared `bool`, not anything truthy |
| `assertNull($a)` | the value is `null` |
| `assertCount($array, $n)` | the array holds exactly `$n` entries |
| `assertThrows($body, C::class)` | running the callable throws `C` or a subclass (`expectException`); the throw is consumed, and the failure names what was thrown instead |
| `assertDoesNotThrow($body)` | running the callable returns |
| `expectFailure($body)` | an assertion inside the callable failed; those failures are discharged from the ledger, and a body in which nothing failed is itself a failure |
| `advance($duration)` | not an assertion: moves the clock a `#[Test(at:)]` fixed |

Both sides of a `T`-typed assertion are the same type, so `assertSame(1, "1")` is a compile error.
`Core\Test\Failure` is an ordinary `Throwable`, and that is what makes composite assertions
ordinary code:

```nvs
<?nvs
use Core\Test;

final class Checks {
    public static function assertEven(int $n): void {
        try {
            Test::assertSame($n % 2, 0);
        } catch (Core\Test\Failure $inner) {
            throw new Core\Test\Failure($n . " is odd: " . $inner->message);
        }
    }
}

Checks::assertEven(4);
echo "4 passed", "\n";

try {
    Checks::assertEven(3);
} catch (Core\Test\Failure $f) {
    echo $f->message, "\n";
}

try {
    Test::assertEquals(1, 2, {message: "a fresh counter is one"});
} catch (Core\Test\Failure $f) {
    echo $f->message, "\n";
}

Test::assertThrows(fn (): void => { throw new ParseError("bad input"); }, RuntimeError::class);
echo "a ParseError is a RuntimeError", "\n";
```
```output
4 passed
3 is odd: Core\Test::assertSame failed: `$actual` is 1, `$expected` is 0
a fresh counter is one: Core\Test::assertEquals failed: `$actual` is 1, `$expected` is 2
a ParseError is a RuntimeError
```

Assertions also work under `nvs run`, outside any test: they throw the same `Failure`, with no
ledger to record it.

# Running tests: `nvs test`

`nvs test main.nvs` compiles the program, runs its test table and prints the report above. Tests
live wherever a class does: beside the code in one file, or in an entry file of their own that
`autoload`s or `require`s the code under test — `nvs test tests.nvs` compiles that graph and runs
only the `#[Test]` methods it declares. A suite of several files goes in a directory instead:
`nvs test tests/`, with one file there that requires the code under test.

`nvs test` reads what to run from each path it is given:

- **a `.nvs` file** is a program, and its `#[Test]` methods run;
- **a directory holding `.nvs` files** is one program made of every one of them (below);
- **a `.nvst` file, or a directory holding only `.nvst` files,** is a conformance run: each case is
  one program under `--FILE--` with its expected output under `--EXPECT--` (and a `--TEST--`
  title), and the report is a one-line count. `nvs test tests/conformance` is how this compiler's
  own suite runs.

A directory holding both kinds is refused, and the two kinds never run in one invocation.

- `--format json` writes one JSON document to standard output at the end: `schemaVersion` (`2`), a
  `summary` (`total`, `passed`, `failed`, `skipped`, `flaky`, `durationMs`) and a `tests` array
  with one `{class, method, file, line, column, verdict, durationMs}` per case — `file`, `line` and
  `column` being where the `#[Test]` method's name is written, one-based, the same location
  `nvs check --json` carries for a diagnostic — plus `reason` for a skip, `failures`
  (every message) for a failure, and `attempts` for a flaky test. `--format junit` writes JUnit
  XML: one `<testsuite>` per class, a `<testcase>` per test, `<skipped>`, `<failure>` with the
  first message as its attribute and all of them as its body, and `<flakyFailure>` for a flaky
  one. Under either machine format what the tests themselves `echo` goes to standard error, so
  standard output is the document alone.
- `--list` answers which tests the program declares and where each is written, without running one:
  a line per test under the human format, and under `--format json` a `schemaVersion: 2` document
  whose `listed` array holds one `{class, method, file, line, column}` per test. It carries no
  summary and no verdict — a listing is not a run — and it is what an editor's test tree is
  populated from.
- The exit status is `1` if any test failed, `0` otherwise, in every format.
- `--filter <text>` runs only the tests whose name contains that text, case-sensitively: a `#[Test]`
  method's name is `Class::method` (`Class::method#0` for a data-provider row), so `--filter Class::`
  is a whole-class selector. A class no filtered test belongs to is not announced and its
  `#[Fixture]`s are not built. The same containment rule selects a `.nvst` case by its path (above).

## Which kind of test to write

Write `#[Test]` methods to test your own code. Every other section of this chapter is about them.

| | `#[Test]` method | `.nvst` case file |
|---|---|---|
| What it tests | a function, a method or a class of your program | one whole program |
| What it checks | any value, with the `Core\Test` assertions | the text the program prints |
| What it reports | one line per test, or `--format json` and `--format junit` | one line with the counts |

Write a `.nvst` case only when the printed output of a whole program is the thing to check:

- a command-line script whose output must match an expected text exactly;
- a program moved from PHP. The case holds the PHP version under `--ORACLE--`, and both versions
  must print the same text.

A `.nvst` case cannot check a return value, and `--format`, `--list` and `--update` do not work
with one. If you are not sure, write a `#[Test]` method. The `nvs test` section of
[the nvs command](#tools-cli) describes the `.nvst` format.

## A directory of test files is one program

This is the layout for a suite of more than one file. `nvs test tests/` requires every `.nvs` file
under `tests/`, subdirectories included, in name order, and runs the `#[Test]` methods of the
program they make together. There is no list of test files to keep: the directory is scanned, so
adding a test is adding a file.

One file in the directory requires the application's entry or bootstrap file, and that gives the
whole directory its `autoload` declarations. The test files themselves require nothing, and the
bootstrap works the same wherever its name sorts:

```
tests/
  bootstrap.nvs       <?nvs require '../src/app.nvs';
  OrderTest.nvs       final class OrderTest { #[Test] public function … }
  PriceTest.nvs       final class PriceTest { #[Test] public function … }
```

`nvs test tests/` then runs both classes' tests in one report. A directory holding both `.nvs` and
`.nvst` files is refused.
