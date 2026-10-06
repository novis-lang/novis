---
id: iteration
title: Iteration and generators
summary: what `foreach` accepts — arrays, `Iterable<T>`, `Iterator<T>` — how a class becomes iterable, and how a method with `yield` is a lazy `Iterator<T>`
keywords: foreach, Iterable, Iterator, iterate, advance, current, yield, generator, Generator, yield from, IteratorAggregate, Traversable, ArrayAccess, Countable, iterator_to_array, Core\Arr::from, lazy, finally, ObjectMap, ObjectSet, Heap, Channel
---

# What `foreach` walks

`foreach` accepts three things: an `array<T>`, an object implementing `Iterable<T>`, and an
object implementing `Iterator<T>`. Each of the three gives the value binding the element type `T`:
`as var $v` takes it, and a written `as T $v` is checked against it. Over an array the loop may
bind the key as well (`foreach ($a as var $k => var $v)`, the key a `string`); the other two have
no key. The array forms, `break`, `continue` and the rest of the statement are in the
[statements](#lang-statements) chapter.
<!-- src: `rule:iteration/two-interfaces` -->

```nvs
<?nvs
class Letters implements Iterable<string> {
    public function iterate(): Iterator<string> {
        yield "x";
        yield "y";
    }
}
var $names = ["a" => "Ada", "b" => "Bob"];
foreach ($names as var $k => var $v) {
    echo $k, "=", $v, ";";
}
echo "\n";
foreach (new Letters() as var $letter) {
    echo Core\Str::upper($letter);
}
foreach ($names as string $k => string $v) {
    echo $k;
}
echo "\n";
```
```output
a=Ada;b=Bob;
XYab
```

# The two interfaces

These are the only iteration interfaces, both global and both generic:

```nvs skip
interface Iterator<T> {
    public function advance(): bool;   // move to the next element; false once exhausted
    public function current(): T;      // the element the last advance() moved to
}

interface Iterable<T> {
    public function iterate(): Iterator<T>;
}
```

A `foreach` over an `Iterable<T>` calls `iterate()` once and drains the iterator it answers;
over an `Iterator<T>` it drains that object directly. A user class implements either **at a
concrete type** — `implements Iterator<int>`, `implements Iterable<User>` — and this is the one
place a user declaration writes a type argument; a class cannot declare type variables of its
own. Both interfaces also name parameter and local types: `Iterable<int> $xs`,
`Iterator<int> $it`. An `Iterable`/`Iterator` subject has no key: a `$k =>` binding over one is
a compile error.

## An `Iterator` written by hand, and driven by hand

`advance()`/`current()` is the whole protocol, so a `while` over the two members drains an
iterator exactly as `foreach` does. `foreach` dispatches on the object's runtime class, so a
subclass overriding `current()` changes what a base-typed loop sees.

```nvs
<?nvs
class Walker implements Iterator<string> {
    private int $i = -1;

    public function constructor(private array<string> $items) {}

    public function advance(): bool {
        $this->i = $this->i + 1;
        return $this->i < (Core\Arr::count($this->items) as int);
    }

    public function current(): string {
        return $this->items[$this->i];
    }
}

array<string> $xs = ["a", "b", "c"];
var $w = new Walker($xs);
while ($w->advance()) {
    echo $w->current();
}
echo "\n";
foreach (new Walker($xs) as string $s) {
    echo $s;
}
echo "\n";
```
```output
abc
abc
```

## An `Iterable` hands out a fresh iterator per loop

`iterate()` runs once per `foreach`, so one object is walked twice, or nested inside a loop over
itself, without sharing a cursor. `iterate()` may be a generator body.

```nvs
<?nvs
class Countdown implements Iterable<int> {
    public function constructor(private int $from) {}

    public function iterate(): Iterator<int> {
        int $i = $this->from;
        while ($i > 0) {
            yield $i;
            $i = $i - 1;
        }
    }
}

var $three = new Countdown(3);
foreach ($three as int $outer) {
    foreach ($three as int $inner) {
        echo $outer, $inner, ";";
    }
}
echo "\n";
int $total = 0;
foreach ($three as int $n) {
    $total = $total + $n;
}
echo $total, "\n";
```
```output
33;32;31;23;22;21;13;12;11;
6
```

```nvs error
<?nvs
class Steps {
    public static function upto(int $n): Iterator<int> {
        int $i = 0;
        while ($i < $n) {
            yield $i;
            $i = $i + 1;
        }
    }
}

foreach (Steps::upto(3) as string $k => int $v) {
    echo $k, "=", $v, ";";
}
```
```output
has no key to bind
```

# Generators

A method whose body contains `yield` is a generator. It must declare the return type
`Iterator<T>` (not `Iterable<T>`, not `Generator`), and each `yield expr;` produces one `T`.
Calling it runs none of the body: it allocates the state object and returns it, and the body
runs only as far as each `advance()` asks. A `static` method, an instance method and an
`iterate()` may each be a generator.

```nvs
<?nvs
class Series {
    public static function announced(int $n): Iterator<int> {
        int $i = 0;
        while ($i < $n) {
            echo "make", $i, ";";
            yield $i;
            $i = $i + 1;
        }
        echo "done;";
    }
}

foreach (Series::announced(3) as int $v) {
    echo "use", $v, ";";
}
echo "\n";

Iterator<int> $it = Series::announced(2);
echo "made;";
while ($it->advance()) {
    echo "got", $it->current(), ";";
}
echo "\n";
```
```output
make0;use0;make1;use1;make2;use2;done;
made;make0;got0;make1;got1;done;
```

The rules of a generator body:

- `yield` is a statement, not a value: `$x = yield 1;` is refused, because nothing is sent back
  into a generator. There is no key half — `yield $k => $v;` is refused.
- `return;` ends the sequence early. `return $value;` is refused: a generator has no return
  value to retrieve.
- `yield from` does not exist; write `foreach ($inner as T $v) { yield $v; }`, which stays lazy
  at both stages.
- A generator cannot take an `inout` parameter.
- Each call makes its own state object: two generators from one method advance independently.
- A generator is drained once. A second `foreach` over the same `Iterator` object sees nothing.
- Calling `current()` on a generator before its first `advance()`, or after `advance()` has
  answered `false`, throws `LogicError`. The parked element is readable only inside the protocol,
  so neither point can answer the element type's null payload or the last element again.

```nvs
<?nvs
class G {
    public static function count(int $n): Iterator<int> {
        int $i = 0;
        while ($i < $n) {
            yield $i;
            $i = $i + 1;
        }
    }

    public static function early(bool $stop): Iterator<int> {
        yield 1;
        if ($stop) {
            return;
        }
        yield 2;
    }
}

var $a = G::count(3);
var $b = G::count(3);
$a->advance();
$a->advance();
$b->advance();
echo $a->current(), $b->current(), "\n";

foreach (G::early(true) as int $v) {
    echo $v;
}
echo "|";
foreach (G::early(false) as int $v) {
    echo $v;
}
echo "\n";

Iterator<int> $once = G::count(2);
foreach ($once as int $v) {
    echo $v;
}
foreach ($once as int $v) {
    echo $v;
}
echo "|\n";
```
```output
10
1|12
01|
```

```nvs error
<?nvs
class G {
    public static function inner(): Iterator<int> {
        yield 1;
    }

    public static function outer(): Iterator<int> {
        yield from G::inner();
    }
}
```
```output
`yield from` does not exist
```

```nvs error
<?nvs
class G {
    public static function g(): Iterator<int> {
        yield 1;
        return 5;
    }
}
```
```output
a generator cannot return a value
```

## `finally` in a generator

A `try … finally` around a `yield` runs its `finally` when the body falls off the end, and also
when the generator is abandoned while suspended inside it — a `break` out of the `foreach`, or
the last reference to the state object going away. A generator that was never advanced runs
nothing.

```nvs
<?nvs
class G {
    public static function guarded(int $n): Iterator<int> {
        try {
            int $i = 0;
            while ($i < $n) {
                yield $i;
                $i = $i + 1;
            }
        } finally {
            echo "[closed]";
        }
    }
}

foreach (G::guarded(3) as int $v) {
    echo $v;
}
echo "\n";
foreach (G::guarded(5) as int $v) {
    echo $v;
    if ($v == 1) {
        break;
    }
}
echo "\n";
```
```output
012[closed]
01[closed]
```

## A generator that throws

A `throw` inside the body surfaces at the `foreach` (or the `advance()`) that resumed it, after
the values already yielded, and is caught there like any other throw.

```nvs
<?nvs
class Feed {
    public static function upTo(int $n): Iterator<int> {
        int $i = 0;
        while ($i < $n) {
            if ($i == 2) {
                throw new RuntimeError("bad element 2");
            }
            yield $i;
            $i = $i + 1;
        }
    }
}

try {
    foreach (Feed::upTo(5) as int $v) {
        echo $v, ";";
    }
} catch (RuntimeError $e) {
    echo "caught: ", $e->message, "\n";
}
```
```output
0;1;caught: bad element 2
```

# `Core` collections are `Iterable`

`Core\ObjectSet<T>`, `Core\Heap<T>` and `Core\Task\Channel<T>` are `Iterable<T>`;
`Core\ObjectMap<K, V>` is `Iterable<K>` — a `foreach` over a map binds its keys. A heap yields
in priority order; a channel's loop takes each sent value and ends when the channel is closed.
Their members are in Part B.

```nvs
<?nvs
var $set = new Core\ObjectSet<int>();
$set->add(3);
$set->add(4);
foreach ($set as int $v) {
    echo $v;
}
echo "\n";

var $map = new Core\ObjectMap<string, int>();
$map->set("a", 1);
$map->set("b", 2);
foreach ($map as string $k) {
    echo $k, "=", $map->get($k), ";";
}
echo "\n";

var $heap = new Core\Heap<int>();
$heap->push(3);
$heap->push(1);
$heap->push(2);
foreach ($heap as int $v) {
    echo $v;
}
echo "\n";
```
```output
34
a=1;b=2;
123
```

# Materialising a sequence: `Core\Arr::from`

`Core\Arr::from` takes whatever `foreach` takes — an array, an `Iterable<T>`, an `Iterator<T>` or a
`Core` collection — and answers an `array<T>` with keys `0…n-1`. Its `limit` option stops a generator early. A
collection is drained exactly as `foreach` walks it, so a `Core\ObjectMap` gives its keys.

```nvs
<?nvs
class G {
    public static function squares(int $n): Iterator<int> {
        int $i = 1;
        while ($i <= $n) {
            yield $i * $i;
            $i = $i + 1;
        }
    }
}

array<int> $a = Core\Arr::from(G::squares(4));
echo Core\Arr::count($a), ":", $a[3], "\n";
array<int> $b = Core\Arr::from(G::squares(100), {limit: 2});
echo Core\Arr::count($b), "\n";
```
```output
4:16
2
```

# What does not exist

`ArrayAccess`, `Countable`, `IteratorAggregate`, `Traversable` and the `Generator` class are not
declared — naming one is a compile error. `iterator_to_array`, `count`, `current`, `next`,
`reset` and every other free function do not exist; a generator's own members and `Core\Arr` are
the replacements. Element access on an object (`$obj[0]`) has no interface to implement: give
the class a method.

```nvs error
<?nvs
class Bag implements Countable {
    public function count(): int {
        return 1;
    }
}
```
```output
`Countable` is not declared
```

```nvs error
<?nvs
class G {
    public static function g(): Iterator<int> {
        yield 1;
    }
}

var $a = iterator_to_array(G::g());
```
```output
is not a function that exists
```
