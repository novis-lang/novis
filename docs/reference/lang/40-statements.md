---
id: statements
title: Statements and control flow
summary: expression statements, blocks and local declarations, `if`, the four loops, `switch`, `break`/`continue` with levels, `return`, `try`/`catch`/`finally`, `throw`, `echo`, `unset`, and the PHP statement forms that do not parse
keywords: statement, block, scope, definite assignment, if, elseif, else if, else, endif, alternative syntax, while, endwhile, do while, for, foreach, endforeach, as, key, value, inout, by reference, Iterator, Iterable, switch, case, default, fallthrough, break, continue, break 2, continue 2, levels, return, try, catch, finally, multi-catch, throw, echo, print, unset, exit, yield, goto, label, declare, strict_types, global, static variable
---

# Expression statements, blocks and declarations

A statement is an expression followed by `;`, a declaration, a block, or one of the control-flow forms below. A discarded expression still runs — a call for its effect, an assignment for its write. `;` on its own is an empty statement.

A local is declared once, with a type or `var`, and every local is **function-scoped**: a block `{ … }` groups statements and opens no scope, so a name declared inside one is the same name after it, and declaring it again anywhere in the function — a second `int $n`, a second `for (int $i …)`, a second `catch (… $e)` — is a compile error. "Once" counts declaration *statements*, not executions: a declaration inside a loop body is one statement and runs on every iteration, initialising the local afresh each time. What a block *does* affect is definite assignment: a local may be read only where every path has assigned it, so a declaration inside an `if` cannot be read after the `if`, and `int $n;` with no initializer is readable once both branches assign it. The declaration forms themselves are the types chapter's.

```nvs
<?nvs
int $n = 1;
$n = $n + 1;
Core\Str::upper("discarded");
{
    int $inner = 40;
}
int $late;
bool $flag = true;
if ($flag) { $late = 1; } else { $late = 2; }
echo $n + $inner, " ", $late, "\n";
```
```output
42 1
```

```nvs error
<?nvs
bool $flag = true;
if ($flag) {
    int $x = 1;
}
echo $x;
```
```output
read before any assignment
```

# `if`, `elseif`, `else`

The condition is any expression, read as a condition (the truth table in the expressions chapter); it is not converted to `bool` and need not be one. `elseif` and `else if` are the same thing. A body is one statement, so braces are optional around a single one. PHP's alternative syntax — `if (…): … endif;`, and `endwhile`, `endfor`, `endforeach`, `endswitch` — does not parse.

```nvs
<?nvs
int $n = 2;
if ($n == 1) {
    echo "one\n";
} elseif ($n == 2) {
    echo "two\n";
} else {
    echo "other\n";
}
if ($n > 1) echo "big\n"; else echo "small\n";
string $s = "";
if ($s) { echo "full\n"; } else { echo "empty\n"; }
```
```output
two
big
empty
```

```nvs error
<?nvs
int $n = 2;
if ($n == 2):
    echo "two";
endif;
```
```output
expected an expression
```

# `while` and `do … while`

`while (cond) body` tests first; `do body while (cond);` runs the body once before testing. `continue` in either jumps to the test.

```nvs
<?nvs
int $n = 0;
while ($n < 3) {
    $n++;
}
echo $n, "\n";
int $m = 5;
do {
    echo $m, "\n";
    $m++;
} while ($m < 3);
```
```output
3
5
```

# `for`

`for (init; cond; step) body`. The init clause is **either** one local declaration — `var $i = 0`, or with a written type, `int $i = 0` — **or** a comma-separated list of expressions over locals declared above the loop; never both, and never two declarations. The condition and the step are each a comma-separated list of expressions, any of them empty, so `for (;;)` loops until a `break`. A condition list runs every expression in it and decides on the last, so an assignment written before that last one happens on every test, including the one that ends the loop. The counter is an ordinary function-scoped local: it is readable after the loop with the value that ended it, and a second loop in the same function needs a different name. `continue` runs the step.

```nvs
<?nvs
int $j = 10;
for (var $i = 0; $i < 3; $i++, $j--) {
    echo $i, ":", $j, " ";
}
echo "\n", $i, " ", $j, "\n";
int $k = 0;
for ($k = 5; $k > 3; $k--) {
    echo $k;
}
echo "\n";
for (;;) {
    echo "once\n";
    break;
}
```
```output
0:10 1:9 2:8
3 7
54
once
```

```nvs error
<?nvs
for (int $a = 0, int $b = 0; $a < 1; $a++) { echo $a; }
```
```output
at most one binding
```

# `foreach`

`foreach (subject as var $v)` or `foreach (subject as var $k => var $v)`. Each binding writes `var` or its type — `foreach (subject as T $v)`, `foreach (subject as string $k => T $v)` — and the two may be mixed in one header. A binding with neither, `as $v`, does not parse.

- `var` on the value binding takes the subject's element type `T`, with any qualifier on it such as `tainted`; `var` on the key binding takes `string`. The type is then fixed, exactly as if it were written. A written type is checked against the element type, and is the one to write when the binding must be wider than the element — `int|float $v` over an `array<int>` whose body stores a `float` in `$v`.
- Over an `array<T>`: the value binds as `T` and the key, when bound, is `string` — never `int`. `inout var $v` or `inout T $v` writes each element back into the array; `&$v` does not parse. The loop walks a copy, so appending to or unsetting from the array inside the body does not change what is visited (an `inout` loop walks the array itself).
- Over an `Iterator<T>` or an `Iterable<T>` — a generator, or a class implementing either — the value binds as `T` and there is no key to bind. The iteration chapter owns those interfaces and generators.
- The subject is an expression, but `as` in the header belongs to `foreach`: converting the subject needs parentheses, `foreach (($m as array<int>) as var $v)`. An array literal as the subject is `array<T>` under a `var` value binding when every element has the type `T`, as it is for `var $xs = [...]`; mixed element types or `[]` do not compile. Under a written binding type the literal has no element type, so bind it to a typed local first.
- The body is one statement; `break`, `continue` and `return` leave it as they leave any loop.
- A binding is declared by the loop, and a later `foreach` in the same function may declare the same name again at the same type, whether written or taken by `var`; at another type it is `E0406`.

```nvs
<?nvs
var $prices = ["apple" => 3, "pear" => 5];
foreach ($prices as var $p) {
    echo $p, " ";
}
echo "\n";
foreach ($prices as var $name => var $each) {
    echo $name, "=", $each, " ";
}
echo "\n";
foreach ($prices as inout var $doubled) {
    $doubled = $doubled * 2;
}
echo Core\Json::encode($prices), "\n";
foreach ($prices as string $name => int|float $amount) {
    if ($amount > 6) {
        $amount = $amount * 0.25;
    }
    echo $name, "=", $amount, " ";
}
echo "\n";
foreach (["new", "sale"] as var $tag) echo $tag, " ";
echo "\n";
mixed $m = [7, 8];
foreach (($m as array<int>) as var $v) echo $v;
echo "\n";
```
```output
3 5
apple=3 pear=5
{"apple":6,"pear":10}
apple=6 pear=2.5
new sale 
78
```

```nvs
<?nvs
class Count {
    public static function upTo(int $n): Iterator<int> {
        int $i = 1;
        while ($i <= $n) {
            yield $i;
            $i++;
        }
    }
}
foreach (Count::upTo(3) as int $v) {
    echo $v;
}
echo "\n";
```
```output
123
```

```nvs error
<?nvs
array<int> $a = [1];
foreach ($a as $v) { echo $v; }
```
```output
a `foreach` binding writes its type, or `var`
```

```nvs error
<?nvs
foreach ([1, "two"] as var $x) { echo $x; }
```
```output
`var` cannot infer an array whose elements have different types
```

# `switch`

`switch (subject) { case expr: … default: … }` compares the subject with each `case` in order by `==` — so a `case` whose type is disjoint from the subject's is a compile error — and runs from the first match onward: a body without `break` **falls through** into the next, wherever it is written, and `default` is tried last. `switch (true)` orders conditions. Nothing matching and no `default` does nothing. A `switch` counts as a level for `break` and `continue`; a `continue` inside one continues the enclosing loop.

```nvs
<?nvs
string $cmd = "stop";
switch ($cmd) {
    case "go":
        echo "going\n";
        break;
    case "stop":
    case "halt":
        echo "stopping\n";
    default:
        echo "(logged)\n";
}
```
```output
stopping
(logged)
```

# `break` and `continue`

`break` leaves the innermost loop or `switch`; `continue` starts the next iteration of the innermost loop. `break N` and `continue N` count enclosing loops **and `switch`es** outward from 1, PHP's way, so `continue 2` inside a `switch` inside a loop is that loop's next iteration — and so is a plain `continue` there, because a `switch` has nothing to continue. `N` is an integer literal, and a level with no matching statement — `break` outside any loop, `break 3` under one loop, `continue` under only a `switch` — is a compile error.

```nvs
<?nvs
for (int $i = 0; $i < 4; $i++) {
    switch ($i) {
        case 1:
            continue;
        case 2:
            break;
        default:
            echo "d", $i, " ";
    }
    echo "t", $i, " ";
}
echo "\n";
array<string> $outer = ["a", "b"];
array<string> $inner = ["x", "y", "z"];
foreach ($outer as string $o) {
    foreach ($inner as string $in) {
        if ($in == "y") { continue 2; }
        if ($o == "b") { break 2; }
        echo $o, $in, " ";
    }
    echo "never\n";
}
echo "done\n";
```
```output
d0 t0 t2 d3 t3
ax done
```

```nvs error
<?nvs
int $n = 0;
break;
```
```output
not inside a loop or `switch`
```

# `return`

`return expr;` leaves a method with a value; `return;` leaves a `void` one, and a `void` method returning a value is a compile error. At the top level of a file, `return` ends that file — the value is what `require` answers (the programs chapter), and in the entry file it ends the program.

```nvs
<?nvs
class Find {
    public static function first(array<int> $xs, int $min): ?int {
        foreach ($xs as int $x) {
            if ($x >= $min) {
                return $x;
            }
        }
        return null;
    }
}
array<int> $xs = [1, 5, 9];
echo Find::first($xs, 4) as string, "\n";
return;
echo "not reached\n";
```
```output
5
```

# `try`, `catch`, `finally`

```nvs skip
try { … }
catch (SomeError $e) { … }      // any number, tried in order; the first whose class matches runs
catch (Throwable) { … }         // the variable may be omitted
finally { … }                   // optional; runs however the try was left
```

- A `catch` names **one** class or interface and matches it and every subclass; the first matching clause wins, so a supertype written first shadows the clauses after it. A `catch (A | B $e)` clause is not available; write two clauses.
- Each `catch` variable is a declared local of the function, so two clauses in one function use two names. Reusing one is **`E0406`** — the ordinary declare-once rule of the types chapter, not a `catch`-specific one, because a clause's `$e` is the same kind of name any other declaration makes.
- `finally` runs on every exit from the `try` and its `catch`es — normal completion, a `return`, a `break` or `continue` out of an enclosing loop, and a throw — and nested `finally` blocks run innermost first.
- A throw inside a `catch` leaves through `finally` to the next enclosing `try`. A `try` with neither `catch` nor `finally` is accepted and merely runs its body.
- The exception classes, their properties and `throw new … {previous: $e}` are the errors chapter's.

```nvs
<?nvs
class Parse {
    public static function run(int $n): string {
        try {
            if ($n == 0) { throw new LogicError("zero"); }
            if ($n == 1) { throw new ArithmeticError("one"); }
            return "ok";
        } catch (LogicError $logic) {
            return "logic:" . $logic->message;
        } catch (Throwable $rest) {
            return "other:" . $rest->message;
        } finally {
            echo "[finally ", $n, "] ";
        }
    }
}
echo Parse::run(0), "\n";
echo Parse::run(1), "\n";
echo Parse::run(2), "\n";
```
```output
[finally 0] logic:zero
[finally 1] other:one
[finally 2] ok
```

Two clauses of the same `try` reaching for one name is the declare-once rule, reported where the
second declaration is:

```nvs error
<?nvs
class Twice {
    public static function run(): string {
        try {
            throw new LogicError("x");
        } catch (LogicError $e) {
            return "a";
        } catch (Throwable $e) {
            return "b";
        }
    }
}
echo Twice::run(), "\n";
```
```output
E0406
```

```nvs
<?nvs
int $i = 0;
while ($i < 3) {
    $i++;
    try {
        if ($i == 2) {
            continue;
        }
        echo "body", $i, " ";
    } finally {
        echo "fin", $i, " ";
    }
}
echo "\n";
```
```output
body1 fin1 fin2 body3 fin3
```

# `catch` as an expression

```nvs skip
expr catch (SomeError $e) => value          // one guarded expression, one arm
expr catch (A $a) => x catch (B $b) => y    // arms of the same guard, tried in order
```

- The arms are clauses of the **one** guard, not guards of each other: `f() catch (A) => x catch (B) => y` tries `A` then `B` against what `f()` threw, and `x` is not guarded by the `B` arm. A supertype written first shadows the arms after it, as in the block form.
- `catch` binds tighter than assignment and looser than the ternary level, so `$x = $a / $b catch (ArithmeticError) => 0` guards the whole division and assigns the whole guard, and a `??` chain or a `?:` is taken whole by the guard and by an arm body alike.
- The class, the optional variable and the refusal of `catch (A | B $e)` are the block form's. The binding is a local of the enclosing function and it ends with its arm.
- The result is the **union** of the guard's type and every arm's, checked against the position the whole expression sits in — neither side against the other. `Repo::get($id) catch (IOError) => null` is a `?int` where `get` returns `int`.
- An arm holds an **expression**, so `throw` is in and `return`, `break` and `continue` are out: **`E0126`**, which names the block form. A `throw` arm produces no value, so it leaves the union alone and the failure leaves the expression.
- What no arm matched leaves carrying the same object, to the enclosing `try` or out of the program. There is no `finally` here — that stays the block form's, and an enclosing one runs as it does for any other throw.
- `catch (Throwable) => value` — the root class, no binding, no `throw` — warns **`W1006`**: it discards every failure, including the ones the site never anticipated. Bind the value, name the class you expected, or write the block form.

```nvs
<?nvs
class Repo {
    public static function get(int $id): int {
        if ($id < 0) {
            throw new IOError("no such row");
        }
        return $id * 2;
    }
}
echo Repo::get(21) catch (IOError $missing) => 0, "\n";
echo Repo::get(-1) catch (IOError $missing) => 0, "\n";
echo Repo::get(-1) catch (IOError $e) => $e->message, "\n";
mixed $either = Repo::get(-1) catch (LogicError $logic) => "logic" catch (IOError $io) => -1;
echo $either, "\n";
```
```output
42
0
no such row
-1
```

An arm that wants a statement has found the block form:

```nvs error
<?nvs
class Repo {
    public static function get(int $id): int {
        return $id;
    }
}
int $n = Repo::get(1) catch (IOError $e) => return 0;
echo $n, "\n";
```
```output
E0126
```

# `throw`

`throw expr;` raises a `Throwable`; it is also an expression (the expressions chapter). Uncaught, it ends the program with status 1 and a backtrace on standard error.

```nvs exit=1
<?nvs
echo "before\n";
throw new RuntimeError("boom");
```
```output
before
```

# `echo`, `print`, `unset`, `exit`, `yield`

- `echo a, b, …;` writes each value in order with nothing between; `print expr;` writes one. Both render a value the way `.` does.
- `unset($a["k"], …)` removes array entries, and only those: a local can never become undeclared again, and a property can never become uninitialized, so `unset($x)` and `unset($o->p)` are compile errors. Assign `null` to a nullable instead.
- `exit;`, `exit(status);`, `exit(message);` end the program (the programs chapter).
- `yield value;` is a statement inside a method returning `Iterator<T>`; it produces no value (`$x = yield …` does not compile) and `yield from` does not exist. The iteration chapter owns generators.

```nvs
<?nvs
echo "a", 1, 2.5, true, null, "\n";
print "p\n";
array<int> $a = ["k" => 1, "j" => 2, "m" => 3];
unset($a["k"], $a["j"]);
echo Core\Json::encode($a), "\n";
```
```output
a12.51
p
{"m":3}
```

```nvs error
<?nvs
int $x = 1;
unset($x);
```
```output
takes an array element of a named holder
```

# Statement forms that do not parse

| PHP | Novis |
|---|---|
| `goto label;`, `label:` | restructure with a loop, an early `return` or a flag |
| `declare(strict_types=1);`, `declare(ticks=…)` | nothing to declare: every file is strict, and the line is a syntax error |
| `if (…): … endif;` and the other `end…` forms | braces |
| `global $x;` | pass a parameter, or a `static` property |
| `static $n = 0;` inside a method | a `private static` property |
| `list($a, $b) = …` | `[int $a, int $b] = …` |
| `include`, `require_once` | `require` |
| `class`, `interface`, `enum`, `type`, `namespace`, `use`, `autoload` inside a body | file scope only |

```nvs error
<?nvs
int $n = 0;
while ($n < 3) {
    $n++;
    if ($n == 2) { goto done; }
}
```
```output
`goto` is not supported
```
