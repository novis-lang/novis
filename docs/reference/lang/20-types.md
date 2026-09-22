---
id: types
title: Types, declarations and conversions
summary: every type, how a binding declares one, every literal, the `as` conversion and its table, implicit widening, narrowing, truthiness, and the `tainted`/`secret` qualifiers
keywords: bool, int, uint, float, decimal, string, bytes, array<T>, callable, class<T>, class reference, mixed, object, nullable, ?T, union, literal type, enum, shape, object literal, type alias, void, never, self, static, iterable, intersection, var, declaration, inout, variadic, default parameter, constant, literal, heredoc, nowdoc, interpolation, duration, html, markup, template, template literal, page, escape, xss, as, conversion, cast, (int), (string), (float), (bool), (array), intval, strval, floatval, boolval, settype, gettype, is_int, is_string, is_array, is_null, is_numeric, widening, narrowing, is, truthy, falsy, tainted, secret, resource
---

# Every binding has a type

Nothing in Novis is untyped. A local, a parameter, a return, a property, a constant and a `foreach`
binding each write a type, and a binding's type never changes after its declaration.

```nvs
<?nvs
int $count = 1;
string $label;
$label = "n=";
$count = $count + 1;
var $ratio = 0.5;
$ratio = $ratio * 2;
echo $label, $count, " ", $ratio, "\n";
```
```output
n=2 1
```

- `T $x = expr;` declares `$x` with type `T`. `T $x;` declares it without a value; reading it before
  every path has assigned it is a compile error.
- A local is declared once. Every later `$x = …` is an assignment and carries no type. Declaring the
  same name again is refused, and so is assigning a name that was never declared.
- `var $x = expr;` infers the type from the initializer and fixes it forever: `var $n = 41;` makes an
  `int`, and `$n = "x"` is then refused. `var` refuses a bare array literal, because `[1, 2]` on its
  own has no element type — write `array<int> $xs = [1, 2];`.
- Locals are function-scoped: a name declared inside an `if` or a loop body is visible after it, and
  the counter a `for` header declares stays visible after the loop. A `catch (T $e)` binding is a
  declaration too, so two `catch` blocks in one function name two different variables.
- A value that does not match the declared type is refused where it is written — `int $i = null;`,
  `uint $u = -1;`, `int $i = 3; string $s = $i;` are all compile errors. The one implicit
  conversion is listed under `Widening without as` below; everything else is spelled `as`.

```nvs error
<?nvs
int $x = 1;
int $x = 2;
```
```output
already declared
```

```nvs error
<?nvs
int $count = 1;
$count = "two";
```
```output
expected `int`, found `string`
```

```nvs error
<?nvs
var $xs = [1, 2, 3];
```
```output
cannot infer an array literal's element type
```

# Numbers: `bool`, `int`, `uint`, `float`, `decimal`

| Type | Holds | Notes |
|---|---|---|
| `bool` | `true`, `false` | `true` and `false` are also literal types of their own |
| `int` | 64-bit signed integer | overflow throws `ArithmeticError`, never wraps |
| `uint` | 64-bit unsigned integer | `0` to `18446744073709551615`; underflow throws |
| `float` | IEEE 754 double | prints without a trailing `.0`; `7 / 2` is `3.5`, `6 / 3` is `2` |
| `decimal` | exact decimal, 28 places | keeps its scale: `1.10 * 3` is `3.30`; division rounds at the 28th place |

- A numeric literal is untyped until it is placed: `uint $u = 6;` and `float $f = 3;` and
  `decimal $d = 1.10;` each read the literal at the declared type. Beside a `uint` operand a bare
  digit run is a `uint` too, so `$u + 1` compiles. A leading `-` is an operator, so `-1` is never a
  `uint`.
- `int` and `uint` never mix: `$i + $u` is a compile error, and neither assigns to the other —
  convert one side with `as`.
- Arithmetic with a `float` on either side promotes the other side to `float`. Arithmetic between
  `decimal` and `float` is a compile error; `decimal` and `int` mix, answering `decimal`.
- `bool` takes part in no arithmetic and converts to nothing but `string` and `bool`;
  `true as int` is a compile error — write `$b ? 1 : 0`.

```nvs
<?nvs
int $i = 9223372036854775807;
uint $u = 18446744073709551615;
float $f = 7 / 2;
echo $i, " ", $u, " ", $f, "\n";
try {
    echo $i + 1, "\n";
} catch (ArithmeticError $e) {
    echo $e->message, "\n";
}
decimal $price = 19.99;
echo $price * 3, " ", 1 as decimal / 3, " ", "0.1" as decimal + "0.2" as decimal, "\n";
```
```output
9223372036854775807 18446744073709551615 3.5
Integer addition overflowed
59.97 0.3333333333333333333333333333 0.3
```

```nvs error
<?nvs
decimal $d = 1.10;
float $f = 0.5;
echo $d + $f, "\n";
```
```output
`decimal` and `float` have no representable common type
```

# Text: `string` and `bytes`

A `string` is always valid UTF-8 and is measured in graphemes — user-perceived characters — so a
combining sequence counts once. A `string` has no subscript: `$s[0]` is a compile error, and a
character or a range is `Core\Str::at` / `Core\Str::slice`. Binary data is the separate `bytes`
type, measured in bytes. There is no `bytes` literal: a UTF-8 constant is `"…" as bytes`, and
arbitrary octets come from `Core\Encoding::fromHex` or `Core\Encoding::fromBase64`.

```nvs
<?nvs
string $s = "grüße";
echo Core\Str::length($s), " ", Core\Bytes::length($s as bytes), " ", Core\Str::length("e\u{301}"), "\n";
bytes $b = "nvs" as bytes;
bytes $raw = Core\Encoding::fromHex("ff00");
echo Core\Encoding::toHex($b), " ", Core\Bytes::length($raw), " ", $b as string, "\n";
try {
    echo $raw as string, "\n";
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
```
```output
5 7 1
6e7673 2 nvs
cannot convert `bytes` to `string`: not well-formed UTF-8 at byte 0
```

`string as bytes` always succeeds; `bytes as string` checks the encoding and throws on malformed
input. A `bytes` value has no string form of its own: `echo $b` is refused, `echo $b as string`
is the spelling. Strings compare with `==` only for equality; ordering is `Core\Str::compare`, and
`"a" < "b"` is a compile error. `"3" * 2` is refused: no arithmetic on text.

# `array<T>`

An `array<T>` is an ordered hash with `string` keys and values of one type `T`. It nests to any
depth (`array<array<int>>`), and it has value semantics: assigning or passing one copies it as far
as anyone can observe, and a write through one binding never shows through another.

```nvs
<?nvs
array<int> $a = [10, 20];
$a[5] = 50;
$a[] = 60;
foreach ($a as string $k => int $v) {
    echo $k, "=", $v, ";";
}
echo "\n", $a[1], $a["1"], "\n";
array<int> $copy = $a;
$copy["x"] = 1;
echo Core\Arr::count($a), " ", Core\Arr::count($copy), "\n";
array<array<int>> $grid = [[1, 2], [3]];
echo $grid[0][1], " ", Core\Json::encode($a), "\n";
```
```output
0=10;1=20;5=50;6=60;
2020
4 5
2 {"0":10,"1":20,"5":50,"6":60}
```

- An `int` subscript is the same key as its decimal spelling: `$a[1]` and `$a["1"]` name one
  element, and a `foreach` key binding is always `string`.
- Every element must match `T` where the literal is written: `array<int> $a = [1, "two"];` is a
  compile error. `array<mixed>` holds anything.
- An `array<T>` has no string form (`echo $a` is refused — render it with `Core\Json::encode`), and
  `$a + $b` is a compile error (`Core\Arr::underlay`).

# `callable`, classes, `object`, shapes

- `callable` is satisfied by a closure and nothing else. A closure is written with `fn`; there is no
  callable string. A call through a `callable` variable answers `mixed`, so its result is converted:
  `$f(4) as int`.
- A class, interface or enum name is a type wherever a type is written. `object` is the top of every
  class type: any instance assigns to it, a property is read through it by name, and `is` or
  `as ClassName` gets the class back.
- `class<T>` is a **class reference**: not an instance, but a class itself, where `T` is the class or
  interface every value of the type names. `class<Dog>` widens to `class<Animal>`. Three sites take
  one and nothing else — `new $cls(...)`, `$cls::f(...)` and `$x is $cls`, each refusing a
  bare `string` — and `as` is its only source (below). `Foo::class` is a `string` and stays one. Two
  class references are equal when they are the same class; nothing else is ever equal to one.
- A **shape** `{x: int, y: string}` is a structural object type, and an **object literal**
  `{x: 1, y: "two"}` builds an instance with exactly those fields. A literal with more fields than a
  shape names still satisfies it. A shape type cannot open a statement (`{` there is a block), so
  name it with a `type` alias and declare through the alias.

```nvs
<?nvs
type Point = {x: int, y: int};
class Geo {
    public static function sum(Point $p): int {
        return $p->x + $p->y;
    }
}
class Cell {
    public int $n = 1;
}
Point $p = {x: 1, y: 2};
var $q = {x: 3, y: 4, z: "extra"};
$q->x = 30;
echo Geo::sum($p), " ", Geo::sum($q), " ", $q->z, "\n";
object $o = new Cell();
echo $o->n, " ", ($o is Cell) ? "cell" : "other", " ", ($o as Cell)->n, "\n";
callable $double = fn(int $x): int => $x * 2;
echo $double(4) as int, "\n";
```
```output
3 34 extra
1 cell 1
8
```

# `mixed`

`mixed` is the one unchecked position: it holds a value of any type, and what is done with it is
decided at run time from the value's own tag. A `mixed` may be an operand of arithmetic, a
condition, a subscript base, a receiver of `->`, a `.` operand and the subject of `is` or
`as`. It does **not** widen into anything narrower: a `mixed` handed to a `string` parameter is a
compile error until it is converted.

```nvs
<?nvs
mixed $m = 5;
mixed $row = ["k" => "v"];
mixed $text = "x";
echo $m + 1, " ", $m as string, " ", $row["k"], " ", $text . "y", " ", Core\Str::upper($text as string), "\n";
mixed $bad = "abc";
try {
    echo $bad + 1, "\n";
} catch (Throwable $e) {
    echo "no arithmetic on text\n";
}
```
```output
6 5 v xy X
no arithmetic on text
```

```nvs error
<?nvs
mixed $text = "x";
echo Core\Str::upper($text), "\n";
```
```output
expected `string`, found `mixed`
```

# Nullable, union, literal and enum-case types

- `?T` holds `T` or `null`. `null` on its own is a type too, and `int $i = null;` is refused.
- `A|B` holds either. `int|string $id` takes an `int` or a `string`; `object|int` is legal.
- A **literal type** is one written value: `"read"|"write"`, `1|2|3`, `true`, `false`. A binding
  of a literal union accepts only those values, checked at compile time for a literal and at run
  time for an `as`.
- An **enum case** is a type of its own: `Mode::Read|Mode::Write $m`. Enums are named integers —
  `enum Mode { Read, Write }` counts from `0`, `enum Mode: int { Read = 1 }` declares the values —
  and `$m as int` reads the number, `1 as Mode` converts back (throwing when no case matches).
  `Mode::Read as string` is refused.
- `iterable` and an intersection `A&B` are accepted in a parameter or return position, but no value
  is assignable to either, so neither can hold anything.

```nvs
<?nvs
enum Mode: int {
    Read = 1,
    Write = 2,
}
?int $maybe = null;
int|string $id = "abc";
"read"|"write" $mode = "read";
1|2|3 $level = 2;
Mode::Read|Mode::Write $m = Mode::Write;
echo $maybe ?? -1, " ", $id, " ", $mode, $level, " ", $m as int, " ", (1 as Mode) == Mode::Read ? "read" : "other", "\n";
try {
    Mode $none = 9 as Mode;
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
echo (9 as ?Mode) == null ? "null" : "case", "\n";
```
```output
-1 abc read2 2 read
`9` is not one of `Mode::Read`, `Mode::Write`
null
```

```nvs error
<?nvs
"read"|"write" $mode = "read";
$mode = "delete";
```
```output
expected `"read"|"write"`, found `string`
```

```nvs error
<?nvs
array<int> $xs = [1, 2];
iterable $i = $xs;
```
```output
expected `iterable`, found `array<int>`
```

# `void`, `never`, `self`, `static`

`void` and `never` are return types only; neither names a parameter or a binding, and `var` is
refused the type of a call that hands nothing back. A `void` call is not an operand: `echo
Helper::nothing()` is a compile error. `never` declares a method that does not come back — it throws,
or it ends the program — and a call to one is not read as an exit, so a caller with another path
still has to return a value on that one. `self` is the declaring class and `static` the called class,
in a return position; neither is a value (`return self;` is refused — write `self::member`).

```nvs
<?nvs
class Base {
    public static function make(): static {
        return new static();
    }
    public function same(): self {
        return $this;
    }
    public function name(): string {
        return "base";
    }
    public static function log(string $m): void {
        echo $m, "\n";
    }
}
final class Child extends Base {
    public function name(): string {
        return "child";
    }
}
Base::log("start");
echo Child::make()->name(), " ", Base::make()->same()->name(), "\n";
```
```output
start
child base
```

```nvs error
<?nvs
class Helper {
    public static function nothing(): void {}
}
echo Helper::nothing();
```
```output
`void` has no string form
```

# `type` aliases

`type Name = T;` gives a type expression a `PascalCase` name, written at file scope beside `namespace`
and `use` or as a member of a class, interface or enum body — never inside a method body, a block or a
closure, where it is refused by name. A body's alias takes no visibility modifier, is reached as
`Owner::Name` from anywhere and as a bare `Name` inside its owner, and is not inherited. The alias is
a compile-time name only. It may not name a single bare class, interface or enum (`type Bar = Foo;` is
refused: a class has its own name); `?Foo`, a union, a shape and an `array<…>` are all allowed.

```nvs
<?nvs
class User {
    public int $n = 7;
}
type Id = int|string;
type MaybeUser = ?User;
Id $a = 1;
Id $b = "x";
MaybeUser $u = new User();
echo $a, $b, " ", $u?->n, "\n";
```
```output
1x 7
```

```nvs error
<?nvs
class Foo {
    public int $n = 1;
}
type Bar = Foo;
```
```output
may not name a single class
```

# Literals

**Numbers.** `1_000`, `0x1F`, `0b101`, `0o17`; `1.5`, `.5`, `5.`, `1e3`, `1E-2`, `1_000.5`. A
leading-zero form `017` is decimal seventeen, not octal. `true`, `false`, `null`.

**Strings.** A single-quoted literal interpolates nothing and has exactly two escapes, `\\` and
`\'` — every other backslash stands for itself. A double-quoted literal interpolates `$x`, `$a[k]`,
`$a[0]` — the bare form reaches one level, no further — and in braces any expression whose first
token is a variable: `{$o->p}`, `{$a["k"]["j"]}`, `{$o->m()}`, `{$a + $b}` (PHP stops at
variable-rooted chains here; Novis takes the whole expression grammar). A `{` not followed by `$`
is literal text, and PHP's deprecated `${name}` form does not exist. Its escapes are
`\\ \" \$ \n \t \r \v \f \e`, an octal `\0` through `\777`, `\xHH` and
`\u{HHHH}`; an unrecognized one such as `\q` keeps its backslash. An interpolated value takes the
same rule as `echo`: scalars and `null` render, `bytes`, arrays, enum cases and objects without
`Stringable` are refused.

```nvs
<?nvs
class Tag {
    public string $name = "div";
    public function upper(): string {
        return Core\Str::upper($this->name);
    }
}
string $who = "world";
array<string> $row = ["name" => "ann"];
array<int> $n = [10, 20];
Tag $t = new Tag();
echo "hi $who, $row[name], $n[1], {$row["name"]}, {$t->name}, {$t->upper()}, {$n[0] + $n[1]}\n";
echo 'raw $who \t', ' ', 'it\'s', "\n";
echo "tab[\t] quote[\"] dollar[\$who] backslash[\\] cp[\u{41}]\n";
echo "oct[\101] hex[\x41] unknown[\q]\n";
```
```output
hi world, ann, 20, ann, div, DIV, 30
raw $who \t it's
tab[	] quote["] dollar[$who] backslash[\] cp[A]
oct[A] hex[A] unknown[\q]
```

**Heredoc and nowdoc.** `<<<TXT … TXT;` interpolates like a double-quoted string; `<<<'TXT'` keeps
every character. The closing marker's indentation is removed from every line.

```nvs
<?nvs
string $who = "Ada";
echo <<<TXT
    Name: $who
      indented
    TXT;
echo "|\n";
echo <<<'TXT'
  literal $who
  TXT;
echo "|\n";
```
```output
Name: Ada
  indented|
literal $who|
```

**Durations.** A number followed by a unit — `ns`, `us`, `ms`, `s`, `m`, `h`, `d`, `w` — is a
`Core\Time\Duration`, and units chain: `1h30m`. It is an object, so `==` between two durations is
identity; compare with `compareTo`.

```nvs
<?nvs
Core\Time\Duration $wait = 1h30m;
echo $wait, " ", $wait->toSeconds(), " ", 250us->toNanoseconds(), " ", 1d->toSeconds(), " ", $wait->compareTo(90m), "\n";
```
```output
1h30m 5400 250000 86400 0
```

**Arrays and objects.** `[1, 2]` is positional (keys `"0"`, `"1"`), `["k" => $v]` is keyed, and
the two mix. `{x: 1, y: "two"}` is an object literal; as a statement or an arrow body it is written
`({…})`. Both are covered above.

## Markup: the `html` template literal

``html`…` `` is a `Core\Html\Markup`. The text between the backticks is written to a page exactly
as it is, and every hole in it is escaped. It is the way to build a page, or one fragment of a page,
as a value. `Core\Html\Markup` is the only type an HTTP request's `echo`, `<?= ?>` and
`Core\Response::html` write raw; a `string` written there is escaped.

- A hole is written as in a double-quoted string: `$name` bare, or `{$…}` with any expression
  whose **first token is a variable** — `{$user->name()}`, `{$row["title"]}`, `{$a + $b}`. A `{`
  not followed by `$` is text, so a `<style>` or `<script>` block needs no escape. It follows that
  `{Page::TITLE}` and `{Page::render()}` are text and are printed as written: bind the value to a
  local first and write `{$title}`.
- A `string` in a hole is escaped, `tainted` or not. A `Core\Html\Markup` in a hole is written raw.
  A `secret` value in a hole is a compile error.
- `+` joins two `Markup` values; `Core\Html::join` joins a list of them. `.` on a `Markup` is a
  compile error, because the text it would make is escaped again at the sink.
- Every byte between the backticks is kept, indentation and newlines included; nothing is stripped
  the way a heredoc's closing marker strips it. A literal backtick is `` \` ``, and `\{` is a
  literal `{` that opens no hole; a `$name` after it still interpolates, so `\{$n}` prints `{`,
  the value and `}`.
- The delimiter is the backtick, always: `html"…"` is a compile error naming the backtick form.
  `"<p>…</p>" as Core\Html\Markup` converts a string written as a literal and nothing else;
  `Core\Html::escape` and `Core\Html::sanitize` are the two ways a computed string becomes a
  `Markup`.

```nvs
<?nvs
class Page {
    public const string TITLE = "News";

    public static function badge(): Core\Html\Markup {
        return html`<span class="badge">new</span>`;
    }
}
tainted string $name = "<b>Ann</b>";
string $title = Page::TITLE;
Core\Html\Markup $badge = Page::badge();
Core\Html\Markup $head = html`<h1>{$title} {$badge}</h1>`;
Core\Html\Markup $body = html`<p>posted by {$name}, {Page::TITLE}</p>`;
echo $head + $body, "\n";
echo html`<pre>
  kept as written \`</pre>`, "\n";
```
```output
<h1>News <span class="badge">new</span></h1><p>posted by &lt;b&gt;Ann&lt;/b&gt;, {Page::TITLE}</p>
<pre>
  kept as written `</pre>
```

```nvs error
<?nvs
Core\Html\Markup $m = html`<p>x</p>`;
string $s = $m . "!";
```
```output
escaping is not idempotent
```

```nvs error
<?nvs
Core\Html\Markup $m = html"<p>x</p>";
```
```output
written with backticks
```

# Widening without `as`

At a binding — an assignment, an argument, a return, an element — these conversions happen without
being written:

| From | Into |
|---|---|
| `int`, `uint` | `float` (exact; a value above 2^53 throws `ArithmeticError`) |
| `T` | `?T`, any union containing `T`, `mixed` |
| a class | `object`, any parent class or interface it implements |
| a literal type | its base (`"read"` into `string`, `true` into `bool`) |
| an untyped numeric literal | `int`, `uint`, `float` or `decimal`, from the target |

Nothing else is implicit: `int` into `uint` (either way), a number into `string`, a `string` into
a number, a `mixed` into anything, `?T` into `T` — each needs `as` or a narrowing test.

```nvs
<?nvs
class Q {
    public static function show(?int $n, int|string $u, mixed $m, float $x): string {
        return (($n ?? 0) as string) . ($u as string) . ($m as string) . ($x as string);
    }
}
int $i = 3;
float $f = $i;
float $g = 3;
echo Q::show(1, 2, 3, 4), " ", $f, $g, "\n";
```
```output
1234 33
```

```nvs error
<?nvs
int $i = 3;
uint $u = $i;
```
```output
expected `uint`, found `int`
```

# The conversion operator `as`

`expr as T` is the only conversion spelling. It binds tighter than every binary operator —
`7 / 2 as int` is `7 / (2 as int)` — and chains: `$m as int as string`. A conversion that exists
but fails at run time throws: `ArithmeticError` when a number does not fit (a fractional `float`
into `int`, a negative into `uint`, an `int` past 2^53 into `float`, a non-integral `decimal` into
`int`), `RuntimeError` for everything else (a string that is not a number, malformed `bytes`, a
`mixed` of the wrong tag, a value outside a literal or enum-case set, an object of another class, a
name that denotes no class this program declares to be a `T` when the target is `class<T>`).

`expr as ?T` is the same conversion answering `null` instead of throwing. It is refused where
`as T` cannot fail (`$i as ?int` on an `int`) and for a class target (`$o as ?Foo`): an object is
tested with `is`. A `class<T>` target is not a class target and is available: `$name as
?class<Animal>` answers `null` for every name `as class<Animal>` would throw for — one that denotes
no class, and one that denotes a class outside `Animal`'s hierarchy. A written-out `Foo::class`
operand stays decided at compile time under both spellings, so `Rock::class as ?class<Animal>` is
refused rather than answering `null`.

```nvs
<?nvs
echo 7 / 2 as int, " ", (7 / 2) as string, " ", 1 + 2 as float, "\n";
string $digits = "42";
int $n = $digits as int;
echo $n + 1, " ", "1e3" as float, " ", 3.0 as int, " ", true as string, "|", false as string, "|", null as string, "|\n";
float $f = 3.9;
string $word = "abc";
int $neg = -1;
try {
    echo $f as int, "\n";
} catch (ArithmeticError $e) {
    echo $e->message, "\n";
}
try {
    echo $neg as uint, "\n";
} catch (ArithmeticError $under) {
    echo $under->message, "\n";
}
try {
    echo $word as int, "\n";
} catch (RuntimeError $text) {
    echo $text->message, "\n";
}
echo ($word as ?int) ?? -1, " ", ($f as ?int) ?? -1, " ", ($digits as ?int) ?? -1, "\n";
```
```output
3.5 3.5 3
43 1000 3 1|||
cannot convert `float` 3.9 to `int`
cannot convert `int` -1 to `uint`
cannot convert string "abc" to `int`
-1 -1 42
```

`as` is also the only way to obtain a `class<T>`, and there are two doors. A `Foo::class` operand is
decided where it is written — a compile error when `Foo` is not a `T`, never a throw the program has
to reach — while any other `string` is checked at run time against the classes this program declares
to be `T`s.

```nvs
<?nvs
class Animal {
    public function speak(): string {
        return "...";
    }
}
class Dog extends Animal {
    public function speak(): string {
        return "woof";
    }
}
class<Animal> $folded = Dog::class as class<Animal>;
string $name = "Dog";
class<Animal> $picked = $name as class<Animal>;
Animal $pet = new $picked();
echo $pet->speak(), " ", ($pet is $folded) ? "a Dog" : "not", "\n";
string $missing = "Cat";
try {
    class<Animal> $bad = $missing as class<Animal>;
    echo (new $bad())->speak(), "\n";
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
```
```output
woof a Dog
cannot convert to `class<Animal>`: the value does not denote a class that is a `Animal`
```

```nvs error
<?nvs
int $n = 3;
echo (int)$n, "\n";
```
```output
`(int)expr` is not supported
```

```nvs error
<?nvs
int $i = 7;
?int $a = $i as ?int;
```
```output
cannot fail, so it never answers `null`
```

The conversion table. A pair not listed is a compile error naming both types.

| From | To | Rule |
|---|---|---|
| `int` ↔ `uint` | | in range, or throws |
| `int`, `uint` | `float` | exact, or throws above 2^53 |
| `float` | `int`, `uint` | integral and in range, or throws — `3.9 as int` throws, it never truncates |
| `int`, `float`, `string` | `decimal` | exact: a `float` is read at the value it prints as, a `string` must be a whole decimal spelling |
| `decimal` | `int`, `float`, `string` | integral for `int`; exact for the others, or throws |
| `string` | `int`, `uint`, `float` | the whole string must be one numeric spelling: `" 42"`, `"42 "`, `""`, `"0x1A"`, `"12.0" as int` all throw; `"1e3" as float` is `1000` |
| `string`, numbers, `bool`, `null` | `bool` | the truthiness table below, never throws |
| `bool` | `string` | `"1"` / `""` |
| `bool` | `int`, `float` | refused — `$b ? 1 : 0` |
| `int`, `uint`, `float`, `decimal`, `null` | `string` | total; `null` is `""`, a float prints as `echo` prints it |
| `string` ↔ `bytes` | | `string as bytes` total; `bytes as string` throws on malformed UTF-8 |
| `bytes`, `array<T>`, an enum case, `void` | `string` | refused |
| enum | `int`, `uint` | the case's number; `int as Enum` throws when no case has that value |
| enum | `string` | refused |
| `mixed` | anything | by the value's runtime tag; a class target throws when the value is another class |
| `array<T>` | `array<U>` | element by element, throwing at the first element `U` refuses; `array<mixed>` accepts every element |
| `?T` | `T` | throws on `null` |
| a class | `object`, a parent, an interface | free; `object as Foo` tests the runtime class |
| `string` | `class<T>` | the name must denote `T` or a class that is one, or it throws; a written `Foo::class` operand is decided at compile time and never throws |
| `class<U>` | `class<T>` | `U` must be a `T`, tested against the class the reference holds |
| any | a literal or enum-case union | a `mixed` must equal a member; a typed operand converts first, then is tested |
| any | `mixed` | free |

# Narrowing

Inside the branch a test proves, a binding is read at the narrower type. Four spellings narrow:

- `$x == null` / `$x != null` on a `?T`, on whichever edge proves the value present (`!` inverts).
- `$x is C` on a `?C`, a union, `object` or `mixed`, on the true edge only: after
  `if ($pet is Cat) { return …; }` the binding is still `Dog|Cat`, not `Dog`.
- `$x == literal` (or an enum case) on a literal or enum-case union, on the edge that proves it.
- `match (true)` and `switch (true)`: each arm's label is one of the tests above and narrows its
  own body. `default` narrows nothing.

A write to the binding inside the branch drops the narrowing. Narrowing is branch-local; a `?T`
that was tested in one `if` is still `?T` after it, and `->` on an un-narrowed `?C` is refused.

```nvs
<?nvs
class Dog {
    public function speak(): string {
        return "woof";
    }
}
class Cat {
    public function purr(): string {
        return "prr";
    }
}
class Ask {
    public static function nullable(?Dog $d): string {
        if ($d == null) {
            return "none";
        }
        return $d->speak();
    }
    public static function either(Dog|Cat $pet): string {
        if ($pet is Cat) {
            return $pet->purr();
        }
        return ($pet as Dog)->speak();
    }
    public static function mode("read"|"write" $m): string {
        if ($m == "read") {
            "read" $only = $m;
            return $only;
        }
        return "other";
    }
    public static function viaMatch(mixed $v): string {
        return match (true) {
            $v is Dog => $v->speak(),
            $v is Cat => $v->purr(),
            default => "other",
        };
    }
}
echo Ask::nullable(new Dog()), " ", Ask::nullable(null), " ", Ask::either(new Cat()), " ", Ask::either(new Dog()), "\n";
echo Ask::mode("read"), " ", Ask::mode("write"), " ", Ask::viaMatch(new Cat()), " ", Ask::viaMatch(1), "\n";
```
```output
woof none prr woof
read other prr other
```

```nvs error
<?nvs
class Dog {
    public function speak(): string {
        return "woof";
    }
}
?Dog $d = new Dog();
echo $d->speak(), "\n";
```
```output
may be `null`, so `->` cannot reach a member of it
```

# Truthiness

A condition — `if`, `while`, `for`'s middle clause, the ternary `?:` and its short form, and the
operands of `&&`, `||` and `!` — is the one place a value is tested without `as`, and it uses PHP's
table: `false`, `0`, `0.0`, `""`, `"0"`, `[]`, an empty `bytes` and `null` are falsy; every other
value, including `"0.0"`, `" "`, every object and every enum case whatever integer backs it, is
truthy. `x as bool` answers the same table. A `mixed` is tested on its runtime tag. `empty($x)` is
the same test negated and `isset($x)` is a `null` test. Those positions are the whole list: a
`match (true)` label is compared against `true` rather than tested, and every other `bool` position
— a binding, an argument, a return — still wants `as bool`.

```nvs
<?nvs
array<int> $empty = [];
?int $none = null;
echo "0" ?: "falsy", " ", "" ?: "falsy", " ", 0 ?: "falsy", " ", 0.0 ?: "falsy", " ", $empty ? "t" : "falsy", " ", $none ? "t" : "falsy", "\n";
echo "0.0" ?: "falsy", " [", " " ?: "falsy", "] ", -1 ? "t" : "falsy", " ", ("abc" as bool) ? "t" : "falsy", " ", ("0" as bool) ? "t" : "falsy", "\n";
int $i = -1;
if ($i && !$empty) {
    echo "truthy\n";
}
```
```output
falsy falsy falsy falsy falsy falsy
0.0 [ ] t t falsy
truthy
```

# Parameters

A parameter is `T $name`, with an optional default `= literal`, and the last may be variadic —
`T ...$rest`, an `array<T>` inside the body. `inout T $name` binds the caller's variable or
property rather than a copy, and the call writes `inout` again in front of the argument; `&` is
not a by-reference marker anywhere. A `foreach` value binding may be `inout` too. Arguments may be
passed by name: `Sum::bump(inout n: $count)`.

```nvs
<?nvs
class Sum {
    public static function of(int $first, int $second = 2, int ...$rest): int {
        int $total = $first + $second;
        foreach ($rest as int $r) {
            $total = $total + $r;
        }
        return $total;
    }
    public static function bump(inout int $n): void {
        $n = $n + 1;
    }
}
echo Sum::of(1), " ", Sum::of(1, 1), " ", Sum::of(1, 1, 5, 6), "\n";
int $count = 5;
Sum::bump(inout $count);
Sum::bump(inout n: $count);
echo $count, "\n";
array<int> $xs = [1, 2];
foreach ($xs as inout int $v) {
    $v = $v * 10;
}
echo Core\Json::encode($xs), "\n";
```
```output
3 2 13
7
[10,20]
```

# Properties and constants

A property is `visibility [static] T $name [= default];` — the default is a scalar literal, `[]`,
an enum case or a class constant, and a union takes whichever of those its own members admit:
`?string $label = null`, `?string $label = "plain"`, `"read"|"write" $mode = "read"`. A class constant is `visibility const T NAME = literal;`, and the type is written there as it is
everywhere else (`E0246`). Constants are read at their declared type — `const uint WIDTH = 5` is
a `uint`. The classes chapter owns everything else about members.

```nvs
<?nvs
class Limits {
    public const uint WIDTH = 5;
    public const string LABEL = "limits";
    public int $n = 1;
    public ?string $label;
    private static uint $made = 0;
    public function constructor() {
        $this->label = null;
        Limits::$made = Limits::$made + 1;
    }
    public static function made(): uint {
        return self::$made;
    }
}
Limits $l = new Limits();
uint $w = Limits::WIDTH;
echo $w + 1, " ", Limits::LABEL, " ", $l->n, " ", $l->label ?? "none", " ", Limits::made(), "\n";
```
```output
6 limits 1 none 1
```

# Qualifiers: `tainted` and `secret`

A qualifier is written before the type, and only on `string` and `bytes`: `tainted string`,
`secret bytes`, `secret tainted string` (that order only). It is checked at compile time and costs
nothing at run time: a qualified value answers every operation exactly as its plain twin does.

**`tainted`** marks untrusted input. A value is tainted where a declaration says so — a parameter
such as a `#[Command]` method's positional argument declared `tainted string $target`, a local, a
property, a return type. It spreads: concatenation, interpolation and any `Core` member whose
answer is built from the input (`Core\Str::upper`, `slice`, `trim`, `replace`, `split`, …) answer
`tainted` when given `tainted`. A member answering a count or a truth (`Core\Str::length`,
`contains`) takes it and answers plain. A parameter whose content is an instruction — a regex
pattern, a `Core\Str::format` template, a `Core\Bytes::pack` format, a date pattern — refuses it,
and so does every `Core` `string` parameter that is not classified either way. A `tainted` value
is removed by a checked conversion — `as int`, `as uint`, `as float`, `as bool`, `as` an enum —
or by a laundering member for one sink (`Core\Regex::quote`, `Core\Uri::encodeComponent`).
`as string` and `as bytes` keep it. Assigning a `tainted string` to a plain `string` is a compile
error.

```nvs
<?nvs
tainted string $input = "42 apples";
tainted string $upper = Core\Str::upper($input);
tainted string $joined = "got: " . $input;
uint $length = Core\Str::length($input);
string $quoted = Core\Regex::quote($input);
echo $upper, " | ", $joined, " | ", $length, " | ", $quoted, "\n";
tainted string $digits = "42";
int $n = $digits as int;
echo $n + 1, "\n";
```
```output
42 APPLES | got: 42 apples | 9 | 42 apples
43
```

```nvs error
<?nvs
tainted string $input = "a";
string $plain = $input;
```
```output
expected `string`, found `tainted string`
```

```nvs error
<?nvs
tainted string $pattern = "a+";
Core\Regex\Pattern $p = Core\Regex::compile($pattern);
```
```output
expected `string`, found `tainted string`
```

**`secret`** marks a confidential value. Nothing grants it: it exists only where a declaration
spells it. It spreads through concatenation and interpolation the same way, independently of
`tainted`, and the same checked conversions remove it. What refuses a `secret` value at compile
time: `echo` and `print`, a `Throwable`'s message, `Core\Debug::dump` and `Core\Debug::render`,
`Core\Json::encode`, and every `Core` parameter typed plain `string` or `bytes` — including the
ones that admit `tainted` (`Core\Str::length($secret)` is refused). Interpolation and `.` spread
the qualifier to their result, so `echo "Bearer {$token}"` is refused for the same reason the bare
`echo $token` is. What still compiles: `==` between two secrets, or a secret and a string; and an
array literal, which infers `array<mixed>` and drops the qualifier, where a declared
`array<secret string>` keeps it and is refused at `Core\Json::encode` with everything else. Beside a
conversion there is exactly one member that removes it, `Core\Secret::reveal($value, $reason)` with
a `bytes` twin `revealBytes`, which writes down why at the call site and removes `secret` alone — a
value that was also `tainted` stays `tainted`.

```nvs
<?nvs
secret string $token = "hunter2";
secret string $header = "Bearer " . $token;
tainted string $input = "x";
secret tainted string $both = $header . $input;
echo $header == "Bearer hunter2" ? "matches" : "differs", "\n";
secret string $digits = "17";
int $n = $digits as int;
echo $n, "\n";
string $shown = Core\Secret::reveal($token, "showing it in the reference");
echo $shown, "\n";
```
```output
matches
17
hunter2
```

```nvs error
<?nvs
secret string $token = "hunter2";
throw new RuntimeError("bad token " . $token);
```
```output
cannot be passed as a `Throwable` message
```

```nvs error
<?nvs
secret string $token = "hunter2";
Core\Debug::dump($token);
```
```output
cannot be passed to `Core\Debug::dump`
```

```nvs error
<?nvs
secret string $token = "hunter2";
echo "Bearer {$token}\n";
```
```output
cannot be written by `echo`
```

```nvs error
<?nvs
secret string $token = "hunter2";
echo Core\Json::encode($token), "\n";
```
```output
cannot be passed to `Core\Json::encode`
```

# What does not exist

| PHP | Novis |
|---|---|
| `(int)$x`, `(string)$x`, `(float)$x`, `(bool)$x`, `(array)$x` | `$x as int`, … — the cast syntax is refused naming `as` |
| `intval`, `strval`, `floatval`, `boolval` | `as int`, `as string`, `as float`, `as bool` |
| `settype($x, "string")` | refused: a binding's type never changes — convert into a new binding |
| `gettype`, `is_int`, `is_string`, `is_array`, `is_null`, `is_numeric` | no free function exists; test a `mixed` with `is` for a class, `($m as ?int) != null` for a scalar, `== null` for null |
| `resource` | no such type; a handle is a `Core` object |
| a callable string `"Foo::bar"`, `[$obj, "m"]` | refused; a `callable` is a closure written with `fn` |
| `$s[0]` on a string | `Core\Str::at`, `Core\Str::slice` |
| `"3" * 2`, `"3" == 3`, `"a" < "b"` | refused; convert with `as`, order with `Core\Str::compare` |
| `list($a, $b) = …` | `[int $a, int $b] = $pair;` — every leaf typed |
| a `float` silently truncated by `(int)` | `3.9 as int` throws; `as ?int` answers `null` |

```nvs error
<?nvs
mixed $m = 1;
settype($m, "string");
```
```output
`settype` is not supported
```

```nvs error
<?nvs
mixed $m = 1;
echo gettype($m), "\n";
```
```output
no free function has this name
```
