---
id: errors
title: Errors, exceptions and limits
summary: the throwable tree, `throw`/`try`/`catch`/`finally`, what an uncaught throw does, and the fatal limits no `catch` sees
keywords: Throwable, Exception, Error, LogicError, RuntimeError, IOError, ParseError, TimeoutError, RecursionError, ArithmeticError, throw, try, catch, finally, rethrow, previous, backtrace, location, message, getMessage, getPrevious, getTrace, getCode, set_error_handler, set_exception_handler, trigger_error, error_reporting, fatal, FATAL, memory limit, onLimit, capability, fs.read, script.spawn, Core\Fatal, Core\Debug, var_dump, print_r, assert, Core\Test\Failure
---

# The throwable tree

Everything that can be thrown is an object of a class under `Throwable`, and `Throwable` is a
class, not an interface: a user class extends it directly. The tree is fixed and short.

<!-- generated: exceptions -->

- There is no `Exception` and no `Error` class. `class E extends Exception`, `catch (Exception $e)`
  and `new Exception("…")` are each refused as an undeclared name; write `Throwable`,
  `LogicError` or `RuntimeError`.
- `LogicError` is a bug in the program: a `match` with no matching arm, a `Core` member called
  with an argument that cannot be right. `RuntimeError` is the world saying no: a missing array key,
  a value that cannot be converted, a capability that is not granted. `IOError`, `ParseError`,
  `TimeoutError` and `RecursionError` are `RuntimeError`s. `ArithmeticError` is its own branch:
  division and modulo by zero, integer overflow, and a `float` that cannot be converted to `int`
  without loss. `Core\Test\Failure` is a failed assertion.
- Every `Core` member's card names the class it throws for each failure, so a `catch` can be as
  narrow as the card.

```nvs error
<?nvs
class ConfigError extends Exception {
}
```
```output
`Exception` is not declared
```

Which class a built-in failure throws:

```nvs
<?nvs
int $zero = 0;
mixed $text = "abc";
float $half = 2.5;
array<int> $map = ["a" => 1];

try {
    echo 1 / $zero;
} catch (ArithmeticError $div) {
    echo $div->message, "\n";
}
try {
    echo $half as int;
} catch (ArithmeticError $lossy) {
    echo $lossy->message, "\n";
}
try {
    echo $text as int;
} catch (RuntimeError $bad) {
    echo $bad->message, "\n";
}
try {
    echo $map["b"];
} catch (RuntimeError $missing) {
    echo $missing->message, "\n";
}
try {
    echo match ($zero) { 1 => "one" };
} catch (LogicError $noArm) {
    echo $noArm->message, "\n";
}
```
```output
Division by zero
cannot convert `float` 2.5 to `int`
cannot convert this value to `int`
undefined array key `b`
no `match` arm matched the subject
```

# Properties, not accessors

Every throwable has four properties and no accessor methods:

| Property | Type | Holds |
|---|---|---|
| `message` | `string` | the text given to the constructor |
| `previous` | `?Throwable` | the throwable given as `{previous: …}`, or `null` |
| `backtrace` | `array<string>` | one `Class::method() at file:line` per frame the throw unwound out of, innermost first; empty for a throw at a file's top level |
| `location` | `string` | `file:line` of the `throw` — empty until the object is thrown, and rewritten by every throw of the same object |

`ParseError` adds `issues`, an array of `{path, message}` records, one per problem the parser
found. `getMessage()`, `getPrevious()`, `getTrace()`, `getTraceAsString()`, `getFile()`,
`getLine()` and `getCode()` do not exist; each is refused with the property that replaces it.
There is no numeric code anywhere in the tree — a failure is a class, never a number.

```nvs error
<?nvs
try {
    throw new LogicError("boom");
} catch (Throwable $e) {
    echo $e->getMessage();
}
```
```output
`Throwable` has no method named `getMessage`
```

```nvs
<?nvs
class Deep {
    public static function inner(): void {
        throw new LogicError("traced");
    }

    public static function outer(): void {
        Deep::inner();
    }
}

try {
    Deep::outer();
} catch (Throwable $e) {
    echo $e->message, " at ", $e->location, "\n";
    foreach ($e->backtrace as string $frame) {
        echo "  ", $frame, "\n";
    }
    echo $e->previous == null ? "no cause" : "has a cause", "\n";
}
```
```output
traced at main.nvs:4
  Deep::inner() at main.nvs:4
  Deep::outer() at main.nvs:8
no cause
```

# Constructing and subclassing

The constructor of every class in the tree is `constructor(string $message, {previous?: Throwable})`:
one required message and an optional options bag. `new RuntimeError()` with no message is
refused. Both arguments may be passed by name — `message:` and `options:`.

A user class extends any class in the tree, including `Throwable` itself. A subclass that declares
no constructor inherits this one; a subclass with its own constructor calls
`parent::constructor($message)` and may add properties, including promoted ones.

```nvs
<?nvs
class NotFound extends RuntimeError {
    public function constructor(public int $status, string $message, public string $path, ?Throwable $cause) {
        parent::constructor($message, {previous: $cause});
    }
}

class Store {
    public static function open(string $path): string {
        try {
            throw new IOError("disk gone");
        } catch (IOError $io) {
            throw new NotFound(404, "cannot open " . $path, $path, $io);
        }
    }
}

try {
    echo Store::open("/etc/app.ini");
} catch (NotFound $e) {
    echo $e->status, " ", $e->message, " (", $e->path, ")\n";
    echo "caused by: ", $e->previous?->message ?? "nothing", "\n";
}
```
```output
404 cannot open /etc/app.ini (/etc/app.ini)
caused by: disk gone
```

# `throw`

`throw` takes an object of a class under `Throwable`. It is a statement and also an expression, so
it can stand where a value is expected — after `??`, in a `match` arm, in a ternary.

```nvs
<?nvs
class Box {
    public static function first(array<int> $items): int {
        return $items["0"] ?? throw new LogicError("the box is empty");
    }
}

array<int> $none = [];
try {
    echo Box::first($none), "\n";
} catch (LogicError $e) {
    echo $e->message, "\n";
}
```
```output
the box is empty
```

# `try`, `catch`, `finally`

```nvs skip
try {
    …
} catch (IOError $io) {          // the first clause whose class matches, top to bottom
    …
} catch (RuntimeError $r) {      // a supertype matches every subclass; `Throwable` matches all
    …
} catch (LogicError) {           // the binding may be omitted
    …
} finally {                      // always runs, after the try body or the catch that handled it
    …
}
```

- Clauses are tried in order and the first whose class is the thrown class or a supertype of it
  runs. Put narrow classes before wide ones — a `Throwable` clause first would take everything.
- A clause names **one** class. `catch (LogicError | IOError $e)` does not compile; write two
  clauses.
- Each binding is a local of the enclosing function, declared by the clause, so two clauses in the
  same function need two names: `catch (IOError $e)` followed by `catch (LogicError $e)` is
  refused as a second declaration of `$e`. Use the binding inside its own clause only.
- A `catch` may `return`, `break`, `continue`, rethrow its binding, or throw something new; a throw
  from inside a `catch` is not seen by the sibling clauses of the same `try` — it goes to the
  next enclosing `try`.
- `finally` runs however the region is left: after the body, after a `catch`, and on the way out
  through `return`, `break`, `continue` or an unhandled throw. Nested `finally` blocks run
  innermost first. A `return` written inside `finally` does not compile: it would replace whatever
  the region was leaving with — silently discarding a throw in flight — so the result is changed
  in a `catch`, or after the region. The same holds for a `break` or `continue` whose target lies
  outside the `finally`; a loop wholly inside the block keeps both.
- A `try` with neither `catch` nor `finally` is accepted and changes nothing.

```nvs
<?nvs
class Loader {
    public static function load(string $key): string {
        try {
            if ($key == "bad") {
                throw new IOError("cannot read " . $key);
            }
            return "value-of-" . $key;
        } finally {
            echo "checked ", $key, "\n";
        }
    }
}

echo Loader::load("host"), "\n";

try {
    echo Loader::load("bad"), "\n";
} catch (LogicError $wrongClause) {
    echo "not this one\n";
} catch (IOError $io) {
    echo "io: ", $io->message, "\n";
}

try {
    try {
        throw new IOError("inner");
    } catch (IOError $cause) {
        throw new RuntimeError("wrapped", {previous: $cause});
    } finally {
        echo "inner finally\n";
    }
} catch (RuntimeError $outer) {
    echo $outer->message, " <- ", $outer->previous?->message ?? "none", "\n";
}
```
```output
checked host
value-of-host
checked bad
io: cannot read bad
inner finally
wrapped <- inner
```

`finally` on every way out of a loop body:

```nvs
<?nvs
int $i = 0;
while ($i < 3) {
    try {
        $i = $i + 1;
        if ($i == 2) {
            continue;
        }
        if ($i == 3) {
            break;
        }
        echo "body ", $i, "\n";
    } finally {
        echo "finally ", $i, "\n";
    }
}
echo "done\n";
```
```output
body 1
finally 1
finally 2
finally 3
done
```

# Rethrowing

`throw $e;` inside a `catch` throws the same object again. Its `location` moves to the rethrow
site; its `message`, `previous` and class do not change. To add context, throw a new object with
the caught one as `previous`, as `Store::open` above does.

```nvs
<?nvs
class Relay {
    public static function go(): void {
        try {
            throw new IOError("first");
        } catch (IOError $io) {
            throw $io;
        }
    }
}

try {
    Relay::go();
} catch (Throwable $e) {
    echo $e->message, " rethrown at ", $e->location, "\n";
}
```
```output
first rethrown at main.nvs:7
```

# An uncaught throw

A throw that no `catch` handles ends the program with exit status 1. Standard output keeps
everything written before it; standard error gets one log record holding the message, the error's
class and one entry per frame, innermost first, ending with the top-level frame `<script>()`.
`[log] format` picks how that record is written: one JSON line by default, readable text under
`format = "text"`. There is no `set_exception_handler` and no `set_error_handler`:
catch it, or let it end the program.

```nvs exit=1
<?nvs
class Repo {
    public static function get(int $id): string {
        throw new RuntimeError("no row " . $id);
    }
}

echo "before\n";
echo Repo::get(7), "\n";
echo "not reached\n";
```
```output
before
```

What standard error carries for the program above, first by default and then under `[log] format =
"text"`:

```text
{"level":"error","msg":"no row 7","fields":{"class":"RuntimeError"},"nodes":[{"function":"Repo::get","file":"main.nvs","line":4},{"function":"<script>","file":"main.nvs","line":9}]}
```

```text
[error] no row 7
  class => string(12) "RuntimeError"
#0 Repo::get() at main.nvs:4
#1 <script>() at main.nvs:9
```

# Recursion depth

A call stack that grows past its bound throws `RecursionError` at the innermost call. It is an
ordinary `RuntimeError`: a `catch` handles it and the program carries on, and uncaught it ends
the program like any other throw, with one backtrace line per frame.

```nvs
<?nvs
class Down {
    public static function forever(int $n): int {
        return Down::forever($n + 1) + 1;
    }
}

try {
    echo Down::forever(0), "\n";
} catch (RecursionError $e) {
    echo "stopped: ", $e->message, "\n";
}
echo "still running\n";
```
```output
stopped: the call stack is too deep
still running
```

# Fatal errors: limits no `catch` sees

A resource limit is set in `nvs.toml`'s `[limits]` table, whose keys are `memory`, `cpu_time`,
`wall_time` and `max_output`. A program that holds more memory than its `memory` limit is stopped
where it stands with a **`FATAL`**, which is not a `Throwable`: no `catch` clause matches it, no
`finally` runs, and the process exits with status 1 after writing `FATAL: …` naming the limit to
standard error.

The one hook is `Core\Fatal::onLimit`. It registers a callable that runs once when a limit stops
the program, out of a slice of memory reserved for it; the callable receives an array whose
`limit` key names the limit that fired (`memory`, `cpu_time`), and may declare no parameter at
all. A second registration replaces the first. After the handler the program still ends.

```toml file=nvs.toml
[limits]
memory = "8M"
```
```nvs exit=1
<?nvs
Core\Fatal::onLimit(fn (array<string> $report): void => {
    echo "limit handler: ", $report["limit"], "\n";
});

echo "before the cap\n";

array<string> $ballast = [];
int $slabs = 0;
while ($slabs < 64) {
    $ballast[] = Core\Str::repeat("x", 1048576);
    $slabs = $slabs + 1;
}

echo "not reached\n";
```
```output
before the cap
limit handler: memory
```

Two other things are fatal in the same way: a `Core\Task\Channel` `send` on a closed channel, and
a channel built with capacity `0`. Neither reaches a `catch`.

# Capability denials are catchable

Reading a file, writing one, or spawning a script needs a capability granted in `nvs.toml`. A call
without the grant throws a `RuntimeError` whose message names the member, the capability and the
path, over a `help:` line naming `nvs.toml` and the table a grant for that capability is written in
— it is an ordinary throw, so the program can catch it and continue. With no `nvs.toml` at all,
nothing is granted.

```nvs
<?nvs
try {
    echo Core\IO::read("data.txt");
} catch (RuntimeError $denied) {
    // The first line goes on with the full path of `data.txt`.
    echo "denied: ", Core\Str::before($denied->message, " for "), "\n";
    echo Core\Str::after($denied->message, "\n"), "\n";
}
echo "still running\n";
```
```output
denied: Core\IO::read needs the capability `fs.read`
help: grant it in nvs.toml under `[capabilities.fs]`
still running
```

A grant is a list of directory roots under `[capabilities.fs]`; `read` and `write` are separate
capabilities. The path is checked in its canonical spelling, so a symlink or a `..` that leaves
the roots is refused, and so is a path that does not exist.

```toml file=nvs.toml
[capabilities.fs]
read = ["."]
```
```txt file=data.txt
hello from disk
```
```nvs
<?nvs
echo Core\Str::trim(Core\IO::read("data.txt")), "\n";

try {
    Core\IO::write("out.txt", "x");
} catch (RuntimeError $denied) {
    echo "denied: ", Core\Str::before($denied->message, " for "), "\n";
    echo Core\Str::after($denied->message, "\n"), "\n";
}
```
```output
hello from disk
denied: Core\IO::write needs the capability `fs.write`
help: grant it in nvs.toml under `[capabilities.fs]`
```

`spawn script` needs `script.spawn` the same way; the [concurrency](#lang-concurrency) chapter
shows the grant.

# Assertion failures

A `Core\Test::assert…` member that fails throws `Core\Test\Failure`, a direct subclass of
`Throwable`, so a test's own `catch (RuntimeError $e)` never swallows one. Inside a `#[Test]`
method the runner catches it and reports the test as failed; outside one it is an ordinary throw.
The testing chapter covers the runner.

```nvs
<?nvs
try {
    Core\Test::assertTrue(false, {message: "flag should be set"});
} catch (Core\Test\Failure $f) {
    echo $f->message, "\n";
}
```
```output
flag should be set: Core\Test::assertTrue failed: `$actual` is false
```

# Inspecting a value

`Core\Debug::dump` writes one rendering per argument to standard error — never to standard
output, and never into a captured buffer — which is what `var_dump` is for. `Core\Debug::render`
returns the same rendering as text, so it can be echoed or embedded. Control bytes are made
visible, a deep or long structure is elided, a cycle is marked, and a `secret` property is redacted.

```nvs
<?nvs
class Point {
    public function constructor(public int $x, public int $y) {
    }
}

echo Core\Debug::render(new Point(1, 2)), "\n";
echo Core\Debug::render(["a" => true, "b" => "tab\there"]), "\n";
```
```output
Point#1 (2) {
  $x => int(1)
  $y => int(2)
}
array(2) [
  "a" => bool(true)
  "b" => string(8) "tab\there"
]
```
