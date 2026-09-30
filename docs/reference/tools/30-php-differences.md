---
id: php-differences
title: "Coming from PHP: every difference, and what to write instead"
summary: the short list of what changed, every PHP spelling the compiler refuses with its replacement and diagnostic code, what parses but behaves differently, and the dev tools that are built in
keywords: PHP, migration, <?php, function, const, define, global, static, $$var, eval, extract, compact, settype, (int), cast, and, or, xor, ===, !==, list(), include, require_once, trait, __construct, __toString, __get, __set, __call, __invoke, __destruct, goto, declare, strict_types, use as, group use, leading backslash, array(), $s[0], mixed, yield from, use ($x), new class, type test, callable string, resource, unset, $_GET, $_POST, $_SERVER, $GLOBALS, $argv, die, print_r, var_dump, echo, elseif, endif, endforeach, #, ?>, &$x, reference, @, backtick, __DIR__, __FILE__, __LINE__, __CLASS__, PHP_EOL, Exception, getMessage, heredoc, nowdoc, ==, ===, equality, type juggling, strlen, mb_strlen, overflow, PHP_INT_MAX, octal, bcmath, gmp, decimal, preg_match, PCRE, ReDoS, password_hash, password_verify, PHPUnit, PHPStan, Psalm, PHP CS Fixer, PHP_CodeSniffer, Xdebug, Composer, differences, switch from PHP
---

Novis is the PHP you already know with one spelling for each thing. This chapter is the
coming-from-PHP tour, in three parts: the short list first, then every PHP spelling the compiler
**refuses** — each with what to write instead and the diagnostic code the refusal reports, so a
program can be moved across one diagnostic at a time — then what still parses but **behaves
differently**. Built-in functions (`strlen`, `array_map`, `json_encode`, …) are not here: none of
them exists as a free function, and the PHP-to-`Core` crosswalk in Part D maps each one to its
member.

# The short list

Ten changes carry most of the distance between the two languages:

- **Every binding declares a type once** — parameter, property, local, closure parameter — and no
  value ever changes type. `mixed` exists for when you mean it.
- **Every function is a method and every constant is a class constant**, built-ins included. There
  is no global scope and nothing the host populates — no `$_GET`, no `$GLOBALS`, no `global`.
- **The methods of the `Core` classes replace PHP's built-in functions.** They all use one
  argument order and accept named arguments. A failure throws a `Throwable` and never returns
  `false`.
- **One equality.** `==` never converts, `===` does not parse, and comparing two disjoint types
  does not compile.
- **`string` is UTF-8 and counts graphemes**; binary data is the separate `bytes` type. The whole
  `mb_*` split is gone.
- **An array is one insertion-ordered, copy-on-write type whose keys are always `string`**, with a
  declared element type.
- **No `eval`, no references, no magic methods.** The constructor is `constructor`, interception
  does not exist, and `inout` is the one by-reference spelling.
- **Concurrency is built in**: `Core\Task` and channels on one core, and `spawn script` isolates
  that share nothing — in place of `pcntl`, `curl_multi` patterns and shared memory.
- **Security is explicit**: reaching the OS needs a capability grant in `nvs.toml` (`fs.read`,
  `net.connect`), and `tainted`/`secret` are type qualifiers the checker enforces.
- **Testing is a language feature**: `#[Test]` methods and `nvs test`, with no framework to
  install — see `the tools you do not install` below.

This short program uses several of them. The function is a static method, the constant belongs
to the class, and `==` compares two strings without converting them to numbers:

```nvs
<?nvs
class Stock {
    public const string UNIT = "pcs";

    public static function total(array<int> $counts): int {
        int $sum = 0;
        foreach ($counts as int $count) {
            $sum += $count;
        }
        return $sum;
    }
}

echo Stock::total(["apples" => 12, "pears" => 5]), " ", Stock::UNIT, "\n";
echo "12" == "012" ? "equal" : "different", " ", Core\Str::length("Café"), "\n";
```
```output
17 pcs
different 4
```

Everything below is the same list at full resolution. Two rules explain most of the refusal
tables. **Every binding declares a type once** — a parameter, a property, a local, a closure
parameter — and no value ever changes type. **Every function is a method and every constant is a
class constant**, so there is no global scope for anything to live in and nothing the host
populates.

<!-- primer -->
# Files, tags and names

| PHP | Novis | Code |
|---|---|---|
| `<?php` | `<?nvs` — the only code-mode tag; `<?=` still works | `E0229` |
| `<?` short tag | none; the file stays in HTML mode and the tag is copied to the output | — |
| `namespace A;` then `use A\B as C;` | `use A\B;` — an import cannot be renamed; use the short name or the full path | `E0212` |
| `use A\{B, C};` | one `use` per name | `E0238` |
| `namespace A { … }` (the braced form), two namespaces in one file | `namespace A;` once, before any declaration — one file is one namespace | `E0243` |
| `use function …;`, `use const …;` | nothing to import: functions and constants are class members | `E0252` |
| `\Core\Str::length($s)` (leading `\`) | `Core\Str::length($s)` — a name with a `\` in it is already absolute | `E0240` |
| `$obj->{$name}`, `$obj->$name` | write the member; hold run-time keys in an `array<T>`, whose keys are `string` | `E0235` |
| `new $className()`, `$className::f()`, `$x is $className` over a `string` | a written class name, or a class reference: `class<T> $cls = $className as class<T>;` then the same three spellings. The `as` is where a name that is not a `T` throws, so every site downstream of it holds a class that already passed | `E0496` at all three |
| `__DIR__`, `__FILE__`, `__LINE__`, `__CLASS__`, `PHP_EOL` | no magic constants; `Throwable::$location` carries a file and line, `"\n"` is the newline | `E0319` |

<!-- primer -->
# Functions, constants and scope

| PHP | Novis | Code |
|---|---|---|
| `function f() { … }` at the top level (or nested) | `class X { public static function f(): T { … } }` | `E0215` |
| `const X = 1;` at the top level | `public const int X = 1;` on a class | `E0216` |
| `define("X", 1)` | the same class constant | `E0320` |
| `global $x;` | pass it as a parameter, or use a `static` property or a constant | `E0204` |
| `static $n = 0;` inside a function | a `private static` property | `E0209` |
| `static fn(…) => …` | `fn(…) => …` — a closure captures `$this` only if it uses it | `E0210` |
| `$$name`, `${"name"}` | an `array<T>`, whose keys are the names | `E0202` |
| `eval($code)` | none: `require` a file, or `spawn script` one | `E0201` |
| `extract($arr)` | destructure: `[int $a, int $b] = $arr;` | `E0205` |
| `compact("a", "b")` | write the array: `{a: $a, b: $b}` or `["a" => $a]` | `E0320` |
| `$_GET`, `$_POST`, `$_SERVER`, `$_COOKIE`, `$_FILES`, `$_ENV`, `$GLOBALS`, `$argv` | no variable is ever populated by the host; a request's data comes through `Core` classes | `E0211` |
| `$x = 1;` with no declaration | `int $x = 1;` or `var $x = 1;` — a local is declared once, with a type | `E0301` |
| `unset($x)` on a local or `unset($o->prop)` | none; `unset` removes an array entry only. Assign `null` where the type is `?T` | `E0234`, `E0413` |
| `class A { const X = 1; }`, `interface I { const X = 1; }` | `public const int X = 1;` — a constant writes its visibility like every member, and its type like every binding | `E0122`, `E0246` |

<!-- primer -->
# Types and conversions

| PHP | Novis | Code |
|---|---|---|
| `function f($x)`, `fn($x) => …` | `function f(int $x)` — every parameter declares a type | `E0101` |
| `public $x;` | `public int $x = 0;` | `E0101` |
| `(int)$s`, `(string)$n`, `(float)`, `(bool)`, `(array)` | `$s as int` — throws where a cast would truncate; `$s as ?int` answers `null` instead | `E0225` |
| `settype($x, "int")` | a new binding: `int $n = $x as int;` | `E0208` |
| `resource` | no such type; a handle is an object of a `Core` class | `E0303` |
| `iterable $x` | `array<T>` or `Iterable<T>` | `E0401` at the call |
| `callable $f = "strlen";`, `callable $f = [$obj, "m"];`, `callable $f = "A::m";` | only a closure is callable: `fn(string $s): uint => Core\Str::length($s)` | `E0418`, `E0419` |
| `never` return type | not lowered in this build (internal error); declare `void` and `throw` | — |
| `public ?int $x = null;` | assign `null` in the constructor; a default is a literal, `[]`, an enum case or a constant | `E0472` |
| `1 == "1"` | convert one side: `$n == ($s as int)` — disjoint types do not compare | `E0466` |
| `"3" * 2`, `"a" < "b"` | `($s as int) * 2`; `Core\Str::compare($a, $b)` | `E0716`, `E0715` |
| `$s++` on a string | none; a binding never changes type | `E0474` |
| `$obj + 1` | objects take part in no arithmetic | `E0716` |
| `array $a` without `<T>` | accepted, but write `array<T>` — the element type is what makes reads typed | — |

<!-- primer -->
# Operators

| PHP | Novis | Code |
|---|---|---|
| `and`, `or` | `&&`, `\|\|` | `E0226` |
| `xor` | `$a != $b` on two `bool`s, or `($a \|\| $b) && !($a && $b)` | `E0226` |
| `===`, `!==` | `==`, `!=` — they never convert, so there is nothing for `===` to add | `E0232` |
| `$a + $b` on arrays | `Core\Arr::underlay($a, $b)` | `E0467` |
| `$s[0]` on a string | `Core\Str::slice($s, 0, 1)` or `Core\Str::at`; `Core\Bytes::slice` for bytes | `E0482` |
| `@expr` | none; failure is a `Throwable`, catch it | `E0236` |
| `` `ls` `` | none; a backtick is not a token, and no member takes a shell string | `E0001` |
| `&$x` in a parameter, a `foreach`, or `$b = &$a` | `inout int $x` at the declaration **and** `f(inout $n)` at the call; `&` is bitwise AND only | `E0237` |
| `f(...$args)` into fixed parameters | only into a `...$rest` variadic; otherwise write the arguments out | `E0489` |
| `$a <> $b` | `!=` — the same comparison, and inequality has one spelling (`rule:expressions/one-equality-operator`) | `E0241` |
| PHP's class-test operator, on every subject | `$x is A`, and `$x is $cls` for a class reference held in a binding — one type test for every type a value can inhabit, which answers rather than refuses when the declaration already settles it (`rule:php-migration/one-type-test`, `rule:types/type-test`) | `E0253` |

`<=>`, `**`, `??`, `??=`, `?:`, `?->` and `.=` all work as in PHP.

# Control flow

| PHP | Novis | Code |
|---|---|---|
| `goto label;` | a loop, an early `return` or a flag | `E0203` |
| `declare(strict_types=1);` | nothing — every program is strict | parse error `E0102` |
| `if (…): … endif;`, `foreach (…): … endforeach;`, `endwhile`, `endswitch` | braces only | parse error `E0102` |
| `die`, `die("msg")` | `exit`, `exit("msg")`, `exit(3)` | `E0228` |
| `include`, `include_once`, `require_once` | `require` — it throws on a missing file and runs every time it is reached | `E0221` |
| `list($a, $b) = $p;` | `[int $a, int $b] = $p;` — every leaf declares its type | `E0230` |
| `yield $k => $v;` | `yield $v;` — an iterator has no key half | `E0448` |
| `yield from $gen;` | `foreach ($gen as T $v) { yield $v; }` | `E0448` |
| `try { … }` with no clause | add `catch (Throwable $e) { … }` or `finally { … }` — PHP refuses this too, and this parser accepted it only by omission | `E0242` |
| `catch (A \| B $e)` | two `catch` clauses, each with its **own** variable name, or one naming a class they both extend — the binding carries one static type | `E0245`; a reused `$e` is `E0406` |
| `$e` still readable after `catch (E $e) { … }` | the binding ends with its clause — only the thrown value assigns it; declare your own variable before the `try` to carry something out | `E0301` |
| `catch (Exception $e)`, `new Exception("x")` | the tree is `Throwable` → `LogicError`, `RuntimeError`, `ArithmeticError`; extend `RuntimeError` | `E0303` |
| `$e->getMessage()` | `$e->message`; also `previous`, `backtrace`, `location` | `E0405` |
| `print "a", "b";` | `print` takes one expression; `echo` takes a list | `E0101` |
| `match` with no matching arm | throws — add a `default` arm; `switch` still falls through without `break` | — |

`elseif` and `else if` both work. `#` starts a line comment (`#[` opens an attribute). A `?>` at
the end of a file is fine.

<!-- primer -->
# Classes

| PHP | Novis | Code |
|---|---|---|
| `function __construct(…)` | `public function constructor(…)` | `E0114` |
| `__toString()` | `implements Stringable` with `toString(): string` | `E0111` |
| `__get`, `__set`, `__isset`, `__unset` | a property hook (`public int $x { get => …; set (int $v) => …; }`) or `implements PropertyObserver`; an undeclared property is always an error | `E0111` |
| `__call`, `__callStatic` | none; declare the method | `E0111` |
| `__invoke` | a closure, `fn(…) => …` | `E0111` |
| `__destruct`, `__clone`, `__sleep`, `__wakeup`, `__serialize`, `__debugInfo`, `__set_state` | none: no destructors, no clone hook, no serialization hook | `E0111` |
| any name starting with `_` | no identifier starts with `_` | `E0111` on a method, `E0112` on a property |
| `function f()` inside a class (no visibility) | `public function f(): T` — every member writes `public`, `protected` or `private` | `E0122` |
| `trait T {}`, `use T;` inside a class | an interface method with a body for behaviour; `class C implements I by $field { … }` for state | `E0227` |
| a property inside an `interface` body | a method every implementor writes, or a typed constant the implementor overrides and a default method reads as `static::NAME` | `E0254` |
| `new class { … }` | a named class in the same file, or a closure where the class is one method — an anonymous class has no name for the static class table to hold | `E0244` |
| `readonly class A` | not a class modifier; `readonly` on a property parses | parse error `E0102` |
| a `readonly` property initialized from any method of the declaring class, the second write throwing at run time | written by that class's `constructor` and nowhere else, refused where the write is written | `E0782` |
| a write from outside the class to a property with a `get` hook and no `set` hook, which PHP stores when the property is backed | only the declaring class writes it — from outside, the accessors are the property | `E0787` |
| `enum E: string { case A = "a"; }` | `enum E: int { A = 1 }` or `enum E { A, B }` — cases only, no `case` keyword, no methods or constants inside | `E0219` for the backing, `E0239` for `case`, `E0220` for a method or constant |
| `class order_line`, `function Total_Price()`, `const maxLines` | `PascalCase` class, `camelCase` member, `SCREAMING_SNAKE_CASE` constant — casing is a hard error | `E0110`–`E0113` |

Constructor promotion (`public function constructor(public int $x)`), `static::`, `parent::`,
`self::`, `abstract`, `final`, interfaces with constants and default method bodies, `clone`,
`new A` without parentheses and `A::class` all work.

<!-- primer -->
# Closures and callables

| PHP | Novis | Code |
|---|---|---|
| `function (int $x) use ($k) { … }` | `fn(int $x): int => $x + $k;` — every outer variable read is captured by value, no `use` clause | `E0222`, `E0223` |
| `function (int $x) { … }` (no `use`) | `fn(int $x): int => { …; return …; }` — the block-body form | `E0222` |
| `fn($x) => $x` | `fn(int $x) => $x` — parameters declare a type unless the closure is passed straight to a parameter that gives one, as in `Core\Arr::map($a, fn($x) => $x * 2)`; the return type may be inferred | `E0808` |
| `strlen(...)` | there is no free function to name: write a closure, `fn(string $s): uint => Core\Str::length($s)`; `A::f(...)` and `$o->m(...)` work as in PHP | `E0320` |
| `call_user_func($f, 1)` | `$f(1)` — the answer of a call through `callable` is `mixed`, so `$f(1) as int` | `E0320` |

<!-- primer -->
# Arrays and strings

| PHP | Novis | Code |
|---|---|---|
| `$a = [1, 2];` | `array<int> $a = [1, 2];` — a variable is declared with its type before it is assigned | `E0301` |
| `var $a = [1, 2];` | `array<int> $a = [1, 2];` — `var` cannot find the element type of an array literal | `E0414` |
| `foreach ([1, 2] as $n)` | bind the literal to a typed local first | `E0401` |
| `array(1, 2)` | accepted; `[1, 2]` is the usual spelling | — |
| `["a" => $x] = $arr;` keyed destructuring | give each variable its type: `["a" => int $x] = $arr;` | parse error `E0101` |
| `print_r($v)`, `var_dump($v)`, `var_export($v)` | `Core\Debug::dump($v)` — writes to standard error, never to the output | `E0320` |
| `"$name"`, `"$a[0]"`, `"{$a[0]}"`, `"$o->x"`, `"{$o->x}"` | all interpolate as in PHP | — |
| `<<<EOT … EOT;`, `<<<'EOT' … EOT;` | both work, with PHP's interpolation rule for each | — |

# What parses but behaves differently

Moving a spelling across without a diagnostic does not yet mean it behaves the same. These are the
changes that survive the parser. <!-- src: docs/divergences.md is the register; the ADR each row cites there is the rule -->

Values and comparison:

| PHP | Novis |
|---|---|
| `"1" == "01"` is `true` — strings juggle to numbers | `==` on two strings compares text: `"01" == "1"` is `false` |
| `==` on arrays ignores key order; on objects it walks properties | arrays compare element by element, in order; objects compare by identity |
| `<`/`>` on two objects walks declared properties | ordering an object needs its class to implement `Comparable`; otherwise it does not compile |
| `PHP_INT_MAX + 1` quietly becomes a `float` | integer overflow throws `ArithmeticError` |
| one integer type; a leading zero is octal, so `017` is fifteen | `int` and `uint` are distinct, and `017` is decimal seventeen — octal is spelled `0o17` |
| `(int)9.9` is `9` | `9.9 as int` throws — rounding is written out: `Core\Math::floor(9.9) as int` |
| `strlen("héllo")` is `6` — bytes; character work needs `mb_*` | `Core\Str::length("héllo")` is `5` — grapheme clusters; raw bytes live in `bytes` |
| array keys are `int` or `string`, and `$a[1]` juggles into `$a["1"]` | keys are always `string`: `$a[1]` *means* `$a["1"]`, and the element type is declared once |
| an enum case is a singleton object with methods and `::cases()` | an enum is a closed set of named integers; a case is a compile-time constant |
| money is `float`, `bcmath` strings or `gmp` | `decimal` is a built-in scalar; `bcmath` and `gmp` are gone |

The library:

| PHP | Novis |
|---|---|
| `require_once` bookkeeping decides whether a file runs | `require` throws on a missing file and runs it every time control reaches it |
| a `preg_*` pattern may backtrack exponentially (ReDoS) | patterns run on a linear-time engine by default; backtracking is opt-in per pattern and budgeted |
| `password_hash` takes an algorithm and cost at the call site | no algorithm argument exists — `Core\Password` owns the choice, `verify` still reads a PHP-stored bcrypt hash, and `needsRehash` answers *weaker*, never *different* |
| control bytes written to a terminal pass through | every control byte reaching the terminal is substituted with a visible glyph |

Four of those rows, run:

```nvs
<?nvs
echo "01" == "1" ? "eq" : "ne", "\n";
echo Core\Str::length("héllo"), "\n";
echo 017, "\n";
try {
    int $n = 9223372036854775807;
    $n = $n + 1;
    echo "wrapped", "\n";
} catch (ArithmeticError $e) {
    echo "overflow throws", "\n";
}
```
```output
ne
5
17
overflow throws
```

# The tools you do not install

A PHP project of consequence carries a second `composer.json` worth of dev tooling. The jobs those
packages do are built into this toolchain or into the language itself:

- **PHPUnit** → `#[Test]` methods, `Core\Test` assertions and `nvs test` are the framework — data
  rows, fixtures, skip-with-reason, retries, JUnit and JSON output. See [testing](#lang-testing).
- **PHPStan / Psalm** → `nvs check` is the compiler, and it already checks what their strictest
  levels check: every binding typed, every member resolved at compile time, every path returning,
  every property initialized — plus `tainted`/`secret` flow, which is Psalm's taint mode as a type
  rule. There are no levels and no baseline file, because there is no untyped code to bridge.
- **PHP CS Fixer / PHP_CodeSniffer** → the style rules that catch bugs are compile errors here —
  identifier casing, a written visibility on every member, one spelling per construct — with no
  suppression and nothing to configure.

# Two spellings side by side

```nvs error
<?nvs
function total(array $xs) {
    return array_sum($xs);
}
echo total([1, 2]) === 3 and true;
```
```output
E0215
```

```nvs
<?nvs
class Cart {
    public const int FREE_SHIPPING = 50;

    public static function total(array<int> $prices): int {
        int $sum = 0;
        foreach ($prices as int $p) {
            $sum += $p;
        }
        return $sum;
    }
}

array<int> $prices = [20, 35];
int $total = Cart::total($prices);
string $input = "7";
int $qty = $input as int;

echo $total, " ", ($total >= Cart::FREE_SHIPPING && $qty > 0) ? "ships free" : "pays", "\n";
echo Core\Str::slice("Novis", 0, 1), " ", 7 <=> 3, " ", 1 == 1.0 ? "eq" : "ne", "\n";

[int $first, int $second] = $prices;
var $double = fn(int $n): int => $n * 2;
echo $first, " ", $second, " ", $double($qty) as int, "\n";

try {
    throw new RuntimeError("stock");
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
```
```output
55 ships free
N 1 eq
20 35 14
stock
```

```nvs error
<?nvs
class Point {
    public function __construct(public int $x, public int $y) {}
}
```
```output
E0114
```

```nvs error
<?nvs
string $name = $_GET["name"];
echo (int)$name;
```
```output
E0211
```
