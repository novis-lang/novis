---
id: expressions
title: Expressions and operators
summary: every operator with its precedence and what it accepts, calls and closures, `match`, arrays and object literals in expression position, and the PHP spellings that do not parse
keywords: operators, precedence, associativity, arithmetic, +, -, *, /, %, **, pow, concatenation, ., .=, ==, !=, ===, !==, <>, <=>, spaceship, comparison, <, <=, >, >=, &&, ||, !, and, or, xor, ??, ??=, ?:, elvis, ternary, ?->, nullsafe, match, is, new, clone, throw expression, print, isset, empty, unset, closure, fn, function, use, callable, first-class callable, named arguments, spread, ..., variadic, inout, array literal, subscript, append, [], destructuring, list(), object literal, shape, ++, --, increment, bitwise, &, |, ^, ~, <<, >>, shift, overflow, ArithmeticError, division by zero, @, backticks, eval, extract, compact, settype, variable variables, $$, =&, reference, |>, pipeline, pipe, $_, hole, substitution
---

# Precedence and associativity

Highest first. A row binds tighter than every row below it.

| Operators | Associativity |
|---|---|
| `->` `?->` `::` `[…]` `(…)` postfix `++` `--` `as T` | left to right |
| `new`, `clone` | — |
| `**` | right |
| prefix `-` `+` `~` `++` `--` | — |
| `\|>` | left |
| `is` | left |
| `!` | — |
| `*` `/` `%` | left |
| `+` `-` | left |
| `<<` `>>` | left |
| `.` | left |
| `<` `<=` `>` `>=` | left |
| `==` `!=` `<=>` | left |
| `&` | left |
| `^` | left |
| `\|` | left |
| `&&` | left |
| `\|\|` | left |
| `??` | right |
| `? :` | right |
| `catch (T $e) => …` | — |
| `=` `+=` `-=` `*=` `/=` `%=` `**=` `.=` `??=` `&=` `\|=` `^=` `<<=` `>>=` | right |
| `print`, `throw`, `yield` | take everything to their right |

- `as` binds tighter than every binary and prefix operator: `-$s as int` is `-($s as int)`, `$s as int ** 2` is `($s as int) ** 2`, and a converted quotient needs parentheses, `($a / $b) as int`. The conversion itself is the types chapter's.
- `**` is right-associative and binds tighter than a prefix sign: `-2 ** 2` is `-4`, `2 ** 3 ** 2` is `512`.
- `!` binds looser than `is`: `!$o is C` is `!($o is C)`.
- `.` binds looser than `+`, `-`, `*` and the shifts: `"sum:" . 1 + 2` is `sum:3`.
- `|>` binds tighter than every binary operator and looser than unary: `-$a |> Core\Math::abs($_)` is `Core\Math::abs(-$a)`, `"x=" . $a |> Core\Str::upper($_)` is `"x=" . Core\Str::upper($a)`, and `$x = $a |> Core\Str::trim($_)` assigns the whole pipeline. Its own section below has the form.
- `new C()->m()` needs no parentheses; `clone $a->b` clones `$a->b`.
- A nested ternary without parentheses groups to the right: `$a ? 1 : $b ? 2 : 3` is `$a ? 1 : ($b ? 2 : 3)`.
- An expression `catch` sits between the ternary and assignment, so `$x = $a / $b catch (ArithmeticError) => 0` guards the whole division and assigns the whole guard, and a following `catch` is the next **arm of the same guard** rather than a guard over the arm before it. The statements chapter has the form.

```nvs
<?nvs
echo -2 ** 2, " ", 2 ** 3 ** 2, " ", 1 + 2 * 3, "\n";
echo "sum:" . 1 + 2, " ", 1 << 2 . "x", "\n";
string $s = "3";
echo -$s as int, " ", $s as int ** 2, " ", 2 * $s as int, "\n";
```
```output
-4 512 7
sum:3 4x
-3 9 6
```

# Operators PHP has that do not parse

| PHP | Novis |
|---|---|
| `and`, `or`, `xor` | `&&`, `\|\|`; `xor` is `$a != $b` on two `bool`s |
| `===`, `!==` | `==`, `!=` — the one equality never converts, so there is nothing for a third `=` to add |
| `(int)$x`, `(string)$x`, … | `$x as int`; the types chapter |
| `@expr` | nothing to suppress: a failure is a `Throwable` |
| `` `cmd` `` | `Core\Process` |
| `$a = &$b`, `&$x` | no references: `inout` parameters, or an object to share |
| `$$name`, `${expr}` | an `array<string, T>` keyed by name |
| `\|>` | not an operator; the token does not parse |
| `$a + $b` on arrays | `Core\Arr::underlay($a, $b)` |
| `$s[0]` on a string | `Core\Str::at`, `Core\Str::slice` |
| `$s++` on a string | no string increment; a binding never changes type |

`<>` is accepted as a second spelling of `!=`.

```nvs error
<?nvs
int $n = 1;
if ($n === 1) { echo "same"; }
```
```output
`===` is not supported
```

# Arithmetic

The operands of `+ - * / % **` are `int`, `uint`, `float` and `decimal`, and nothing else: a `bool`, a `string`, `bytes`, an array, an enum case, `null` and an object are refused where they are written. Nothing converts on its own — `"3" * 2` does not compile; write `($s as int) * 2`.

| Operands | Result |
|---|---|
| `int ⊕ int`, `uint ⊕ uint` | the same type; overflow **throws** `ArithmeticError` — no wrap, no promotion to `float` |
| `int / int`, `uint / uint` | `int\|float`: `6 / 3` is `2`, `7 / 2` is `3.5`. The union binds to a declared `float`; `int $n = 7 / 2` is a compile error. `Core\Math::intDiv` is integer division |
| either side `float` | `float` for `+ - * / **`; `%` is a compile error (`Core\Math::mod` is the float remainder) |
| `int ⊕ uint` | **compile error** — convert one side with `as int` / `as uint`. An integer literal beside a `uint` is a `uint` |
| `decimal ⊕ decimal`, `decimal ⊕ int` | `decimal`, exact; division rounds at the widest scale the result admits |
| `decimal ⊕ float` | **compile error** — a `float` literal beside a `decimal` is a `float`, so `$d + 0.2` is refused; write `$d + (0.2 as decimal)` |
| `%` | integers only, sign of the dividend; `Modulo by zero` throws |
| `**` | an integer result for integer operands — a negative exponent throws unless the base is `1` or `-1`; `2 ** 0.5` is a `float` |
| `/ 0` | `Division by zero` throws, whatever the operand types — the zero divisor is refused before they are consulted, so there is one rule and not two. `Core\Math::fdiv` is IEEE's `INF` where you want it |

```nvs
<?nvs
echo 7 / 2, " ", 6 / 3, " ", 7 % 3, " ", (-7) % 3, " ", 2 ** 10, "\n";
echo 1.5 + 2.25, " ", 2.5 * 4.0, " ", 3 + 0.5, " ", 10 ** 18, "\n";
float $avg = 7 / 2;
echo $avg, " ", Core\Math::intDiv(7, 2), "\n";
uint $u = 3;
echo $u * 2, " ", $u / 2, "\n";
decimal $tenth = 0.1;
decimal $fifth = 0.2;
echo $tenth + $fifth, " ", $tenth * 3, "\n";
```
```output
3.5 2 1 -1 1024
3.75 10 3.5 1000000000000000000
3.5 3
6 1.5
0.3 0.3
```

```nvs
<?nvs
int $big = 9223372036854775807;
try { echo $big + 1, "\n"; } catch (ArithmeticError $a) { echo $a->message, "\n"; }
try { echo 1 / 0, "\n"; } catch (ArithmeticError $b) { echo $b->message, "\n"; }
try { echo 1 % 0, "\n"; } catch (ArithmeticError $c) { echo $c->message, "\n"; }
try { echo 2 ** -1, "\n"; } catch (ArithmeticError $d) { echo $d->message, "\n"; }
uint $zero = 0;
try { echo $zero - 1, "\n"; } catch (ArithmeticError $e) { echo $e->message, "\n"; }
try { echo 1.0 / 0, "\n"; } catch (ArithmeticError $f) { echo $f->message, "\n"; }
echo Core\Math::fdiv(1.0, 0.0), "\n";
```
```output
Integer addition overflowed
Division by zero
Modulo by zero
Negative exponent has no integer result
Integer subtraction overflowed
Division by zero
INF
```

```nvs error
<?nvs
int $i = 1;
uint $u = 2;
echo $i + $u;
```
```output
no representable common type
```

## Unary operators and increment

Prefix `-`, `+` and `~` take a number only — `+"3"` is refused, not converted. `++`/`--` are a write of `± 1` to an `int`, `uint`, `float` or `decimal` local, property or array element; the prefix form answers the new value, the postfix form the old one, and the same overflow rule applies.

```nvs
<?nvs
int $n = 3;
echo -$n, " ", +$n, " ", ~$n, " ", $n++, " ", $n, " ", ++$n, " ", --$n, "\n";
float $f = 1.5;
$f++;
echo $f, "\n";
```
```output
-3 3 -4 3 4 5 4
2.5
```

## Bitwise operators and shifts

`& | ^ ~ << >>` take `int` or `uint` and answer the operand type; both sides of a binary one must have the same signedness, the shift count included. `>>` is arithmetic on `int` and logical on `uint`. A shift by a negative count throws `ArithmeticError`; a count past the width answers `0` (or the sign fill for `int >>`).

```nvs
<?nvs
echo 6 & 3, " ", 6 | 3, " ", 6 ^ 3, " ", ~5, " ", 1 << 3, " ", -8 >> 1, "\n";
uint $all = 18446744073709551615;
uint $one = 1;
echo $all >> $one, " ", ~$one, "\n";
int $n = 1;
try { echo $n << -1, "\n"; } catch (ArithmeticError $e) { echo $e->message, "\n"; }
echo $n << 64, "\n";
```
```output
2 7 5 -6 8 -4
9223372036854775807 18446744073709551614
Bit shift by negative number
0
```

# Comparison and equality

`==` and `!=` are the whole of equality. Neither converts anything, and two operands whose static types can never hold the same value do not compile — `1 == "1"`, an enum case against an `int`, a non-nullable type against `null`, `string` against `bytes`, two unrelated classes. Convert one side first: `$s == ($n as string)`.

| Type | Equal when |
|---|---|
| `int`, `uint`, `float`, `decimal` | mathematically equal, in any pairing: `1 == 1.0`; `decimal` ignores scale, `1.10 == 1.1` |
| `string` | the same code points — `"1" == "01"` is false, and nothing is ever numeric |
| `bytes` | the same bytes |
| `array<T>` | same length, same keys in the same order, every element equal by this table |
| object | **the same instance** — a `clone` or a second `new` is not equal |
| enum case | the same case |
| `?T` against `null` | the null test; it narrows in the branch where it held |
| `mixed` | decided at run time from the tags; different kinds answer `false` |

`< <= > >= <=>` order numbers (any pairing of the four numeric types, exactly, with no conversion), two `bool`s (`false < true`), and two objects of one class that implements `Comparable`. Everything else is refused where it is written: two strings order through `Core\Str::compare`, an enum through `as int`, and arrays, `bytes`, `callable` and `null` do not order at all. `<=>` answers `-1`, `0` or `1` as an `int`.

```nvs
<?nvs
echo (1 == 1.0) as string, "|", ("1" == "01") as string, "|", (2 < 3.5) as string, "|", (false < true) as string, "|\n";
echo 3 <=> 5, " ", 5 <=> 3, " ", 3 <=> 3, "\n";
array<int> $a = [1, 2];
array<int> $b = [1, 2];
array<int> $c = [2, 1];
echo ($a == $b) as string, "|", ($a == $c) as string, "|\n";
echo Core\Str::compare("apple", "banana"), "\n";
```
```output
1||1|1|
-1 1 0
1||
-1
```

```nvs error
<?nvs
int $n = 1;
if ($n == "1") { echo "x"; }
```
```output
disjoint
```

```nvs error
<?nvs
echo ("a" < "b") as string;
```
```output
has no meaning for `string`
```

```nvs
<?nvs
class Money implements Comparable {
    public function constructor(public int $cents) {}
    public function compareTo(self $other): int {
        return $this->cents <=> $other->cents;
    }
}
Money $a = new Money(5);
Money $b = new Money(9);
Money $same = $a;
echo ($a == $b) as string, "|", ($a == $same) as string, "|", ($a < $b) as string, "|", $a <=> $b, "\n";
```
```output
|1|1|-1
```

# Logical operators and truth

`&&` and `||` short-circuit and answer a `bool`; `!` negates. Their operands are read as conditions: `0`, `0.0`, `""`, `"0"`, `[]`, `null` and `false` are false and everything else is true, PHP's table — the one place a value is tested without `as bool`. `and`, `or` and `xor` do not parse.

```nvs
<?nvs
class T {
    public static function say(string $s, bool $v): bool { echo $s; return $v; }
}
echo (T::say("L", false) && T::say("R", true)) as string, "|";
echo (T::say("L", true) || T::say("R", false)) as string, "\n";
string $zero = "0";
array<int> $none = [];
echo (!$zero) as string, (!"") as string, (!$none) as string, (!0.0) as string, (!"x") as string, "|\n";
```
```output
L|L1
1111|
```

# String operators

`.` concatenates and `.=` appends; either side may be a `string`, `int`, `uint`, `float`, `decimal`, `bool` (`true` is `1`, `false` is empty), `null` (empty) or an object implementing `Stringable`. `bytes`, an array, an enum case and a `void` call have no string form and are refused — `$b as string`, `Core\Json::encode($a)`. Double-quoted strings interpolate `$name` and `{$expr}`; the literal forms are the types chapter's.

```nvs
<?nvs
echo "a" . 5 . 1.5 . 10.0 . true . false . null, "|\n";
string $s = "x";
$s .= "y";
$s .= 3;
string $who = "Ada";
echo $s, " ", "Hello $who, {$who}!", "\n";
```
```output
a51.5101|
xy3 Hello Ada, Ada!
```

# `??`, `?:`, `?->` and the ternary

- `$a ?? $b` answers `$a` unless it is `null` — or an absent array key, the one read of a missing key that does not throw. It is right-associative, so `$a ?? $b ?? $c` asks each in turn. On a value that can never be `null` it compiles and answers the left side.
- `$a ??= $b` assigns only when `$a` is `null`.
- `$a ?: $b` tests `$a` as a condition and answers it when true, otherwise `$b`.
- `$o?->p` and `$o?->m()` answer `null` when `$o` is `null`; the result is nullable. A `?->` chain cannot be assigned through.
- `$c ? $a : $b` tests `$c` as a condition. The two arms may differ in type; the value is their union, and it widens at the binding (`float $x = $c ? 1 : 2.5`).

```nvs
<?nvs
class P { public int $x = 1; public function get(): int { return $this->x; } }
?int $a = null;
int $c = 3;
echo $a ?? $c, " ", ($a ?? 7) + 1, "\n";
$a ??= 5;
echo $a, "\n";
array<int> $m = ["k" => 1];
echo $m["k"] ?? 0, " ", $m["gone"] ?? 0, "\n";
string $empty = "";
echo ($empty ?: "default"), " ", (0 ?: 4), "\n";
?P $none = null;
?P $some = new P();
echo ($none?->get() == null) as string, " ", $some?->x, " ", $some?->get(), "\n";
int $n = 5;
echo $n > 3 ? "big" : "small", " ", $n > 9 ? "huge" : $n > 3 ? "big" : "small", "\n";
```
```output
3 8
5
1 0
default 4
1 1 1
big big
```

```nvs error
<?nvs
class P { public int $x = 1; }
?P $p = new P();
$p?->x = 2;
```
```output
cannot be written through
```

# Assignment

`=` assigns to a local, a property, a static property or an array element, and is itself an expression whose value is the value written, so `$a = $b = 3` and `int $c = ($a = 5) + 1` both work. Every compound form `$x op= e` is `$x = $x op e` with `$x` evaluated once, and it cannot change the target's type: `$n /= 2` on an `int` is refused, because `/` answers `int|float`. A local is declared once, with a type (the types chapter); a plain `$x = …` to an undeclared name is a compile error.

```nvs
<?nvs
int $a = 0;
int $b = 0;
$a = $b = 3;
int $c = ($a = 5) + 1;
echo $a, $b, " ", $c, "\n";
int $n = 10;
$n += 5; $n -= 3; $n *= 2; $n %= 7; $n **= 3; $n <<= 1; $n >>= 2; $n &= 7; $n |= 8; $n ^= 1;
float $f = 9.0;
$f /= 2;
echo $n, " ", $f, "\n";
```
```output
53 6
12 4.5
```

# `match`

`match (subject) { … }` is an expression. Each arm lists one or more conditions, compared with the subject by `==` — so a condition whose type is disjoint from the subject's is a compile error — and the first arm whose condition holds supplies the value; `default` is taken last wherever it is written. `match (true)` is the ordered-condition form. No arm matching and no `default` throws a `LogicError`; a `match` with no arms at all does not compile. An arm's body is one expression, not a block.

```nvs
<?nvs
int $n = 2;
string $size = match ($n) {
    1, 2 => "low",
    3 => "three",
    default => "other",
};
echo $size, " ", match (true) { $n > 5 => "big", $n > 1 => "mid", default => "small" }, "\n";
echo "n is " . match ($n) { 2 => "two", default => "many" } . "\n";
try {
    echo match ($n) { 1 => "one" }, "\n";
} catch (LogicError $e) {
    echo $e->message, "\n";
}
```
```output
low mid
n is two
no `match` arm matched the subject
```

# `is`, `new`, `clone`, `throw`, `print`, `exit`, `isset`, `empty`

- `$x is T` answers whether `$x` currently holds a `T`, for any type a value can inhabit, and narrows `$x` to `T` in the branch where it held. It is total: a subject whose declared type settles the answer compiles and folds to `true` or `false`. A `$` on the right is the one value form, `$x is $cls`, where `$cls` is a `class<T>`; anything else on the right is a type.
- `new C(args)` constructs; the argument list is a call's, named arguments included, and may be omitted when empty. `new static()` and `new self()` work inside a class. `new C(...)` has no first-class form.
- `clone $o` is a shallow copy of an object: scalar and array properties are copied, object properties are shared. There is no clone hook. `clone` takes an object only.
- `throw expr` is an expression, so it sits on the right of `??`, `||` or a ternary arm; the errors chapter owns what may be thrown.
- `print expr` writes one value and answers `1`.
- `exit`, `exit(status)` and `exit(message)` are expressions too — `$v ?? exit(1)` — and end the program (the programs chapter).
- `isset($a, $b, …)` is true when every operand holds a non-`null` value; each operand must be storage — a local, a property, or an array element, whose absent key answers false. `empty($x)` is `!$x` by the truth table, and an absent key is empty.

```nvs
<?nvs
class Inner { public int $v = 1; }
class Outer {
    public Inner $in;
    public array<int> $list = [];
    public function constructor() { $this->in = new Inner(); $this->list[] = 1; }
}
Outer $a = new Outer();
Outer $b = clone $a;
$b->list[] = 2;
$b->in->v = 9;
echo Core\Arr::count($a->list), Core\Arr::count($b->list), " ", $a->in->v, " ", ($a == $b) as string, "|\n";
mixed $m = new Inner();
if ($m is Inner) { echo "inner ", $m->v, "\n"; }
?int $none = null;
array<?int> $arr = ["k" => null, "j" => 1];
echo isset($none) as string, isset($arr["k"]) as string, isset($arr["j"]) as string, isset($arr["zz"]) as string, "|";
echo empty($none) as string, empty($arr["zz"]) as string, "|", print "p\n";
?string $name = null;
try {
    string $s = $name ?? throw new LogicError("no name");
} catch (LogicError $e) {
    echo $e->message, "\n";
}
```
```output
12 9 |
inner 1
1|11|p
1no name
```

```nvs error
<?nvs
class Box {}
class T {
    public function m(mixed $v, string $name): void {
        bool $b = $v is $name;
    }
}
```
```output
not a class name
```

# Calls

- `Class::m(args)` calls a static method, `$o->m(args)` an instance method; results chain, `Greeter::make()->greet("x")`, and a `new` chains without parentheses.
- Arguments are positional, or named `name: value` in any order, and a named argument may skip a defaulted parameter in the middle. Evaluation is in written order.
- `...$array` spreads an `array<T>` into a variadic `T ...$rest` parameter — only into one; the fixed parameters before it are still written out.
- A parameter declared `inout T $x` is written back to the caller's variable, and the call writes `inout` too: `M::bump(inout $n)`. Only a local can be passed `inout`, not an array element.
- A `callable` value is called through the variable that holds it, `$f(1)`, an element, `$ops["k"](1)`, or a parenthesised property, `($this->cb)(1)`. The call answers `mixed`, so `int $r = $f(1) as int`. Extra arguments are dropped; too few throw at the call.
- A call that returns `void` is a statement and no operand.

```nvs
<?nvs
class Greeter {
    public function constructor(private string $greeting) {}
    public function greet(string $name, string $mark = "!"): string { return $this->greeting . ", " . $name . $mark; }
    public static function make(): Greeter { return new Greeter("Hi"); }
    public static function sum(int ...$xs): int { int $t = 0; foreach ($xs as int $x) { $t += $x; } return $t; }
    public static function twice(inout int $n): void { $n = $n * 2; }
}
echo Greeter::make()->greet("Ada"), " ", new Greeter("Yo")->greet(mark: "?", name: "Bob"), "\n";
array<int> $xs = [4, 5];
int $n = 4;
Greeter::twice(inout $n);
echo Greeter::sum(1, 2, 3), " ", Greeter::sum(...$xs), " ", Greeter::sum(), " ", $n, "\n";
callable $f = fn(int $x): int => $x + 1;
array<callable> $ops = ["dbl" => fn(int $x): int => $x * 2];
echo $f(1), " ", $f(1, 99), " ", $ops["dbl"](4), " ", (fn(int $x): int => $x * 10)(4), "\n";
```
```output
Hi, Ada! Yo, Bob?
6 9 0 8
2 2 8 40
```

# The pipeline operator

`$subject |> RHS` is `RHS` with its hole `$_` replaced by `$subject`, and the **parser** does the replacing. What every later pass sees is the tree the nested spelling writes, so a pipeline types, converts, taints and compiles exactly as the nesting it stands for. No callable is involved, nothing is allocated, and `|>` has no run-time existence at all.

- The right side is parsed as a postfix expression — a call, a subscript, a member access, or a parenthesised group. Anything else is written parenthesised: `$n |> ($_ * 2)`.
- `$_` appears **exactly once** on a right side. A right side with none is `E0129`, a second `$_` on one right side is `E0130`, and a `$_` written anywhere outside a right side is `E0131`.
- `|>` is left-associative: `$a |> f($_) |> g($_)` is `g(f($a))`.
- It binds tighter than every binary operator and looser than unary, so `$a |> Core\Str::length($_) > 5` compares the length and `$x = $a |> Core\Str::trim($_)` assigns the trimmed string.
- This is not PHP 8.5's `|>`, which applies a callable resolved at run time. A first-class callable or a closure on the right side is `E0129`, and the diagnostic says which of the two operators you wrote.

```nvs
<?nvs
string $s = "  Novis  ";
echo $s |> Core\Str::trim($_) |> Core\Str::lower($_), "\n";
int $n = -7;
echo -$n |> Core\Math::abs($_), " ", $s |> Core\Str::length($_) > 5, "\n";
array<string> $xs = ["c", "a", "b"];
echo $xs |> Core\Arr::sort($_) |> Core\Str::join($_, "-"), " ", 5 |> ($_ * 2), "\n";
```
```output
novis
7 1
a-b-c 10
```

```nvs error
<?nvs
string $s = "hi";
echo $s |> Core\Str::upper(...), "\n";
```
```output
the right side of `|>` needs the hole `$_`
```

```nvs error
<?nvs
string $s = "hi";
echo $s |> Core\Str::replace($_, $_, "x"), "\n";
```
```output
`$_` may appear exactly once on the right side of a `|>`
```

```nvs error
<?nvs
string $s = $_;
echo $s, "\n";
```
```output
`$_` is the pipeline hole and has no meaning here
```

# Closures

`fn` is the only closure literal, and a closure is the only value a `callable` holds. `fn(params): T => expr` answers the expression; `fn(params): T => { … }` runs a block and `return`s. Every parameter declares a type; the return type may be omitted. Every outer local the body reads is captured **by value when the closure is created**, and `$this` is captured inside a method. There is no `use (…)` clause, no capture by reference, no `static fn`, no `inout` parameter, and no anonymous `function () {}`. `Class::m(...)` is not a way to obtain a closure — write `fn(...) => Class::m(...)`.

```nvs
<?nvs
class Counter {
    public int $base = 10;
    public function adder(): callable {
        return fn(int $x): int => $this->base + $x;
    }
}
string $prefix = "one";
callable $tag = fn(string $s): string => $prefix . ":" . $s;
$prefix = "two";
callable $sum = fn(array<int> $xs): int => {
    int $t = 0;
    foreach ($xs as int $x) { $t += $x; }
    return $t;
};
array<int> $ns = [1, 2, 3];
Counter $c = new Counter();
echo $tag("x"), " ", $sum($ns), " ", $c->adder()(5), "\n";
```
```output
one:x 6 15
```

```nvs error
<?nvs
callable $f = function (int $x): int { return $x + 1; };
```
```output
anonymous `function` literals are not supported
```

# Arrays in expressions

An array literal is `[a, b]`, `["k" => v]`, or both mixed; a literal needs a declared target type — `array<int> $a = [1, 2]`, never `var $a = [1, 2]`. `[...$a, x]` copies `$a`'s entries, keys included, into the literal. Every key is a `string`: an `int` or `uint` subscript names the same entry as its decimal spelling, so `$a[8]` and `$a["8"]` are one key while `"08"` is another, and a `foreach` key binding is always `string`.

- `$a["k"]` reads; an absent key **throws** a `RuntimeError` (`undefined array key`). `$a["k"] ?? $d` is the read that does not.
- `$a["k"] = v` writes, `$a[] = v` appends at the highest integer key so far plus one (`0` in an empty array), and both reach into nested arrays: `$g["r"]["c"] = 1` creates the inner array. `[]` is only a write target.
- `unset($a["k"])` removes one entry and moves nothing; `isset($a["k"])` is false for an absent key or a stored `null`.
- An array is a value: assignment, passing and returning copy it, and a write through a temporary — `f()["k"] = 1` — does not compile.
- `$a + $b` does not compile; `Core\Arr::underlay` is the same operation.

```nvs
<?nvs
array<int> $a = [10, 20];
$a[] = 30;
$a["k"] = 40;
$a[-5] = 50;
$a[] = 60;
foreach ($a as string $k => int $v) { echo $k, ":", $v, " "; }
echo "\n";
array<int> $b = [...$a, "j" => 70];
unset($a["k"]);
echo Core\Arr::count($a), " ", Core\Arr::count($b), " ", $b["k"], " ", $a[0] + $a["1"], "\n";
array<array<int>> $g = [];
$g["r"]["c"] = 1;
$g["r"][] = 2;
echo Core\Json::encode($g), " ", isset($g["x"]) as string, "|", $g["x"]["y"] ?? 9, "\n";
try {
    echo $a["missing"], "\n";
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
```
```output
0:10 1:20 2:30 k:40 -5:50 3:60
5 7 40 30
{"r":{"c":1,"0":2}} |9
undefined array key `missing`
```

## Destructuring

`[T $a, T $b] = $arr;` reads elements off one array: a positional leaf reads key `"0"`, `"1"`, … by its position, `'k' => T $v` reads a key, an empty slot skips a position, and a `[...]` leaf nests. Every leaf declares its type — there is no `var` leaf — and the source must be a typed array. A leaf whose key is absent throws like the subscript it is. `list(...)` does not parse.

```nvs
<?nvs
array<int> $pair = [1, 2];
[int $a, int $b] = $pair;
[, int $second] = $pair;
array<string> $row = ["id" => "7", "name" => "ada"];
["name" => string $name, "id" => string $id] = $row;
array<array<int>> $pts = [[1, 2], [3, 4]];
[[int $x1, int $y1], [int $x2, int $y2]] = $pts;
echo $a, $b, $second, " ", $id, $name, " ", $x1, $y1, $x2, $y2, "\n";
```
```output
122 7ada 1234
```

```nvs error
<?nvs
array<int> $pair = [1, 2];
list($a, $b) = $pair;
```
```output
`list(...)` is not supported
```

# Object literals

`{name: value, …}` builds an anonymous object with exactly those fields, read and written as `$p->name`; its type is the shape `{name: T, …}`, which a `type` alias, a parameter or a return type can name. Two literals are two objects, so `==` is identity. Where a `{` would otherwise start a block — at the start of a statement, and directly after a closure's `=>` — the literal is written in parentheses, `({…})`. A field name written twice is a compile error.

```nvs
<?nvs
type Point = {x: int, y: int};
Point $p = {x: 1, y: 2};
var $q = {label: "q", at: {x: 3, y: 4}};
callable $mk = fn(int $x): Point => ({x: $x, y: 0});
$p->x = 10;
echo $p->x + $p->y, " ", $q->label, $q->at->y, " ", ($p == $p) as string, ($p == $q) as string, "|\n";
```
```output
12 q4 1|
```

```nvs error
<?nvs
{x: 1};
```
```output
ambiguous with a block
```

# Refused in expression position

Each of these is parsed only so the diagnostic can name the replacement: `eval` (use `require` or `spawn script`), `extract` (destructure or index), `settype` (`as` into a new binding), `compact` and every other PHP free function (a `Core` member — `Core\Str::length($s)`), `$$name` and `${expr}`, `list(…)`, `(int)` casts, `@`, `=&`, backticks, `die` (`exit`), `include`/`require_once` (`require`), `yield` used as a value, and `self`/`static`/`parent` outside a class.

```nvs error
<?nvs
string $name = "value";
string $value = "x";
echo $$name;
```
```output
variable variables
```
