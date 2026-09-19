---
id: concurrency
title: Tasks, channels and isolates
summary: structured concurrency with `Core\Task`, bounded channels, and `spawn script` isolates that share nothing
keywords: Core\Task, all, map, limit, deadline, TimeoutError, sleep, usleep, Core\Task\Channel, send, close, spawn script, await, Core\Script\Handle, isolate, ScriptResult, script.spawn, args, output, capture, inherit, async, await, Fiber, pcntl, pcntl_fork, pthreads, parallel, curl_multi, threads, workers, shared state
---

# No colouring: I/O just waits

There is no `async`, no `await` on a promise, no `Fiber`, no callback style. A member that waits
— `Core\Time::sleep`, a file read, a channel `send` — parks the calling task and returns when it
is done; the code around it is ordinary sequential code. Concurrency is expressed by *running
several tasks*, and every task is a child of the call that started it: the call does not return
while any child is still running.

`async function`, `new Fiber(…)`, `pcntl_fork()` and threads do not exist; `Core\Task::map` is
what `curl_multi_*` was for. Nothing is shared between requests: there are no globals that outlive
one, and no `Core` member that keeps state across them.

# `Core\Task::all`: a fixed set of tasks

`Core\Task::all` takes a shape whose every field is a zero-argument callable, runs them all
concurrently, and answers a shape with the same field names, each typed by that field's declared
return type — no cast at the use site. Where the closure was written does not matter: a literal, a
first-class callable and a variable are equally good, and a field whose callable declares no return
type answers `mixed` for that field alone. What is refused is a subject that is not a shape of
callables.

```nvs
<?nvs
var $page = Core\Task::all({
    rows:  fn(): array<int> => [1, 2, 3],
    label: fn(): string     => "all",
});
echo $page->label, "=", Core\Arr::count($page->rows), "\n";
```
```output
all=3
```

```nvs error
<?nvs
var $r = Core\Task::all([fn(): int => 1]);
```
```output
expected `{name: callable(): T, ...}`
```

# `Core\Task::map`: one task per element

`Core\Task::map` calls a closure once per element of an array, each call its own task, and answers
the results under the input's keys and in the input's order, whatever order they finished in. The
closure receives `($value, $key)` and may declare only the first.

```nvs
<?nvs
array<int> $prices = ["apple" => 3, "pear" => 4];

var $doubled = Core\Task::map($prices, fn(int $p): int => $p * 2);
foreach ($doubled as string $name => int $cents) {
    echo $name, "=", $cents, "\n";
}

var $labels = Core\Task::map($prices, fn(int $p, string $name): string => $name . ":" . $p);
foreach ($labels as string $label) {
    echo $label, "\n";
}
```
```output
apple=6
pear=8
apple:3
pear:4
```

# `limit` and `deadline`

Both members take an options bag `{limit?: uint, deadline?: Core\Time\Duration}`.

- `limit` caps how many children run at once; the rest are queued, never refused. Omitted, every
  child starts at once. A `limit` of `0` throws `LogicError`.
- `deadline` bounds the **whole call**, not each child. When it expires every child still running
  is cancelled, the call waits for those cancellations, and then throws `TimeoutError`. A cancelled
  child runs no further code of its own — not its `catch`, not the statement after its wait.

```nvs
<?nvs
class Gauge {
    public static int $live = 0;
    public static int $peak = 0;
}

Core\Task::map(Core\Arr::range(1, 6), fn(int $n): int => {
    Gauge::$live += 1;
    if (Gauge::$live > Gauge::$peak) {
        Gauge::$peak = Gauge::$live;
    }
    Core\Time::sleep(5ms);
    Gauge::$live -= 1;
    return $n;
}, {limit: 2});
echo "peak ", Gauge::$peak, "\n";

try {
    Core\Task::map([1, 2], fn(int $n): int => {
        Core\Time::sleep(200ms);
        return $n;
    }, {deadline: 20ms});
    echo "finished\n";
} catch (TimeoutError $late) {
    echo "deadline hit\n";
}
```
```output
peak 2
deadline hit
```

# A child's throw

The first child to throw cancels its siblings, and once they are gone the call throws that same
object in the caller — the caller's `catch` sees the child's class and message, not a wrapper.
Nothing of the group is still running when the `catch` runs.

```nvs
<?nvs
try {
    Core\Task::all({
        slow: fn(): int => {
            Core\Time::sleep(2s);
            return 1;
        },
        bad: fn(): int => {
            Core\Time::sleep(10ms);
            throw new RuntimeError("bad gave up");
        },
    });
    echo "not reached\n";
} catch (RuntimeError $e) {
    echo "caught: ", $e->message, "\n";
}
```
```output
caught: bad gave up
```

# `Core\Task\Channel<T>`: a bounded queue between tasks

`new Core\Task\Channel<T>(capacity)` builds a queue that holds `capacity` values. `send` queues one
and **suspends the sender while the channel is full** — the buffer never grows. `close` ends the
stream; values already queued are still delivered, and closing twice changes nothing. A channel is
its own iterator: `foreach ($chan as T $v)` takes values out, waiting while the channel is empty
and open, and ends once it is empty and closed. A producer that never closes leaves its consumer
waiting forever, so close when done.

A `send` on a closed channel, and a capacity of `0`, are fatal errors — the program stops, and no
`catch` sees them (the [errors](#lang-errors) chapter).

```nvs
<?nvs
var $chan = new Core\Task\Channel<int>(2);
int $consumedAtSend = 0;

Core\Task::all({
    produce: fn(): void => {
        foreach (Core\Arr::range(1, 6) as int $i) {
            $chan->send($i);
        }
        $chan->close();
    },
    consume: fn(): int => {
        int $sum = 0;
        foreach ($chan as int $v) {
            $sum = $sum + $v;
            echo "got ", $v, "\n";
        }
        return $sum;
    },
});
echo "done\n";
```
```output
got 1
got 2
got 3
got 4
got 5
got 6
done
```

# Isolates: `spawn script` and `await`

`spawn script 'path.nvs'` runs another file **as if it were its own request**: fresh globals, fresh
class statics, its own heap and its own output buffer, sharing only compiled code and its parent's
budget. It is an expression whose value is a `Core\Script\Handle`; the child starts at once, and
the parent joins it with `await $handle`, an expression that answers the child's result.

```nvs skip
var $job = spawn script 'jobs/report.nvs' with(
    args:   {month: 7},         // a value copied into the child
    output: 'capture',          // 'capture' (the default) or 'inherit'
);
var $result = await $job;
```

- The entry is either a **path** — any `string` expression naming a file, relative to the working
  directory, so a variable or a computed path is fine — or a **static method**, written
  `Class::method(...)`. An `fn` literal is refused (`E0802`): an isolate shares nothing but compiled
  code, and a literal would carry the scope around it across that boundary.
- `with(…)` is optional. `output: 'capture'` collects what the child writes into the result;
  `output: 'inherit'` lets the child write straight to the parent's standard output, interleaved
  with the parent's own lines in whatever order the two run — only after `await` is all of it
  there. Any other spelling throws `LogicError` at the spawn. `args:` takes any value, copies it into
  the child, and the child reads it back with `Core\Script::args()`; a child spawned without the
  option reads `null`. `limits:` sets a sub-cap the child runs under — tighter than what is left of
  the tree's budget, never wider — and `grants:` names the capabilities it holds, intersected with
  the ones its parent holds. `on: 'worker'` starts the child on another core; `on: 'here'`, which is
  also what a spawn with no `on:` does, starts it on the parent's own, and the placement decides
  which core runs it and nothing else about it.
- `await` takes a handle and nothing else; `await $n` on an `int` is a type error. A handle can be
  awaited **once**: a second `await` of the same handle throws `LogicError`.
- Spawning needs the `script.spawn` capability. Without it the spawn throws a catchable
  `RuntimeError` in the parent naming the capability.

The result is a record with four fields:

| Field | Type | Holds |
|---|---|---|
| `ok` | `bool` | `true` when the child ended normally |
| `value` | `mixed` | what the child's top-level `return` answered — convert it with `as`; a child with no `return` answers `null` |
| `output` | `string` | everything the child wrote, when `output` is `'capture'`; empty when inherited |
| `error` | `?{class: string, message: string}` | the class name and message of the child's uncaught throw; `null` when `ok` |

```toml file=nvs.toml
[capabilities.script]
spawn = true
```
```nvs file=adder.nvs
<?nvs
int $total = 0;
foreach (Core\Arr::range(1, 10) as int $n) {
    $total += $n;
}
echo "adding\n";
return ["sum" => $total, "count" => 10];
```
```nvs file=hello.nvs
<?nvs
echo "child says hello\n";
```
```nvs
<?nvs
echo "parent starting\n";
var $adder = spawn script "adder.nvs" with(output: "capture");
var $hello = spawn script "hello.nvs" with(output: "inherit");

var $greeted = await $hello;
echo "hello ok=", $greeted->ok ? "true" : "false", "\n";

var $added = await $adder;
var $answer = $added->value as array<int>;
echo "sum=", $answer["sum"], " count=", $answer["count"], "\n";
echo "captured: ", $added->output;

try {
    var $again = await $adder;
} catch (LogicError $twice) {
    echo $twice->message, "\n";
}
```
```output
parent starting
child says hello
hello ok=true
sum=55 count=10
captured: adding
`await` was given a handle a previous `await` already collected
```

# A child shares nothing

A class static written by the parent reads as its default in the child, and a write in the child
never reaches the parent. The same holds for every global and for output: with `output: 'capture'`
the child's `echo` arrives only as the `output` string.

```toml file=nvs.toml
[capabilities.script]
spawn = true
```
```nvs file=child.nvs
<?nvs
final class Ledger {
    public static int $entries = 0;
}

echo "child read ", Ledger::$entries, "\n";
Ledger::$entries = 99;
```
```nvs
<?nvs
final class Ledger {
    public static int $entries = 0;
}

Ledger::$entries = 7;
var $job = spawn script "child.nvs" with(output: "capture");
var $done = await $job;
echo $done->output;
echo "parent still reads ", Ledger::$entries, "\n";
```
```output
child read 0
parent still reads 7
```

# A child's failure is a value

An uncaught throw inside the child does not cross the boundary as a throw. The parent's `await`
returns normally with `ok` false and `error` holding the class name and message; what the child
wrote before failing is still in `output`. A `RecursionError` in the child arrives the same way.
The parent carries on.

By contrast, a mistake made **by the parent** throws in the parent: a path that names no file or a
file that does not compile throws `RuntimeError`, a bad `output:` spelling throws `LogicError`,
and a missing `script.spawn` grant throws `RuntimeError`.

```toml file=nvs.toml
[capabilities.script]
spawn = true
```
```nvs file=thrower.nvs
<?nvs
echo "child got this far";
throw new RuntimeError("the child could not finish");
```
```nvs
<?nvs
var $bad = spawn script "thrower.nvs" with(output: "capture");
var $failed = await $bad;
echo "ok=", $failed->ok ? "true" : "false", "\n";
echo "class=", $failed->error?->class ?? "none", "\n";
echo "message=", $failed->error?->message ?? "none", "\n";
echo "output=", $failed->output, "\n";

try {
    spawn script "no-such-child.nvs";
} catch (RuntimeError $missing) {
    echo "parent's mistake: ", $missing->message, "\n";
}
echo "parent still running\n";
```
```output
ok=false
class=RuntimeError
message=the child could not finish
output=child got this far
parent's mistake: `spawn script 'no-such-child.nvs'`: `no-such-child.nvs` could not be compiled; see the errors above
parent still running
```

Without a grant, the spawn itself is what throws:

```nvs file=child.nvs
<?nvs
echo "never runs";
```
```nvs
<?nvs
try {
    var $job = spawn script "child.nvs";
    var $done = await $job;
} catch (RuntimeError $denied) {
    echo $denied->message, "\n";
}
```
```output
`spawn script` needs the capability `script.spawn` for child.nvs, which is not granted
help: grant it in nvs.toml under `[capabilities.script]`
```

# What does not exist

- `async`, a promise-style `await`, `Fiber`, generators as coroutines: a task is a closure handed to
  `Core\Task`, and waiting is implicit.
- `pcntl_fork`, `pthreads`, `parallel`, a `spawn worker`: an isolate is the one unit of separate
  execution, and its entry is a whole file or a static method.
- A closure or an `fn` literal as a spawn entry: refused, because a literal would carry the scope
  around it into a child that shares nothing but compiled code.
- State shared between requests, or between an isolate and its parent, other than the values that
  cross at `await`.
