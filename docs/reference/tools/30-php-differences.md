---
id: php-differences
title: "Coming from PHP: what does not parse, and what to write instead"
summary: every PHP spelling the compiler refuses, grouped by theme, each with the Novis spelling that replaces it and the diagnostic code it reports
keywords: PHP, migration, <?php, function, const, define, global, static, $$var, eval, extract, compact, settype, (int), cast, and, or, xor, ===, !==, list(), include, require_once, trait, __construct, __toString, __get, __set, __call, __invoke, __destruct, goto, declare, strict_types, use as, group use, leading backslash, array(), $s[0], mixed, yield from, use ($x), new class, instanceof, callable string, resource, unset, $_GET, $_POST, $_SERVER, $GLOBALS, $argv, die, print_r, var_dump, echo, elseif, endif, endforeach, #, ?>, &$x, reference, @, backtick, __DIR__, __FILE__, __LINE__, __CLASS__, PHP_EOL, Exception, getMessage, heredoc, nowdoc
---

Novis is the PHP you already know with one spelling for each thing. This chapter is the list of
PHP spellings the compiler **refuses**, each with what to write instead and the diagnostic code the
refusal reports, so a PHP program can be moved across one diagnostic at a time. Built-in functions
(`strlen`, `array_map`, `json_encode`, …) are not here: none of them exists as a free function, and
the PHP-to-`Core` crosswalk in Part D maps each one to its member.

Two rules explain most of the table. **Every binding declares a type once** — a parameter, a
property, a local, a closure parameter — and no value ever changes type. **Every function is a
method and every constant is a class constant**, so there is no global scope for anything to live
in and nothing the host populates.

# Files, tags and names

| PHP | Novis | Code |
|---|---|---|
| `<?php` | `<?nvs` — the only code-mode tag; `<?=` still works | `E0229` |
| `<?` short tag | none; the file stays in HTML mode and the tag is copied to the output | — |
| `namespace A;` then `use A\B as C;` | `use A\B;` — an import cannot be renamed; use the short name or the full path | `E0212` |
| `use A\{B, C};` | one `use` per name | `E0238` |
| `namespace A { … }` (the braced form), two namespaces in one file | `namespace A;` once, before any declaration — one file is one namespace | `E0243` |
| `use function …;`, `use const …;` | nothing to import: functions and constants are class members | `E0306` |
| `\Core\Str::length($s)` (leading `\`) | `Core\Str::length($s)` — a name with a `\` in it is already absolute | `E0240` |
| `$obj->{$name}`, `$obj->$name` | write the member; hold run-time keys in an `array<string, T>` | `E0235` |
| `new $className()`, `$className::f()`, `$x instanceof $className` | a written class name only | `E0496`; the first two stop the compiler with an internal error in this build |
| `__DIR__`, `__FILE__`, `__LINE__`, `__CLASS__`, `PHP_EOL` | no magic constants; `Throwable::$location` carries a file and line, `"\n"` is the newline | `E0319` |

# Functions, constants and scope

| PHP | Novis | Code |
|---|---|---|
| `function f() { … }` at the top level (or nested) | `class X { public static function f(): T { … } }` | `E0215` |
| `const X = 1;` at the top level | `public const int X = 1;` on a class | `E0216` |
| `define("X", 1)` | the same class constant | `E0320` |
| `global $x;` | pass it as a parameter, or use a `static` property or a constant | `E0204` |
| `static $n = 0;` inside a function | a `private static` property | `E0209` |
| `static fn(…) => …` | `fn(…) => …` — a closure captures `$this` only if it uses it | `E0210` |
| `$$name`, `${"name"}` | an `array<string, T>` keyed by name | `E0202` |
| `eval($code)` | none: `require` a file, or `spawn script` one | `E0201` |
| `extract($arr)` | destructure: `[int $a, int $b] = $arr;` | `E0205` |
| `compact("a", "b")` | write the array: `{a: $a, b: $b}` or `["a" => $a]` | `E0320` |
| `$_GET`, `$_POST`, `$_SERVER`, `$_COOKIE`, `$_FILES`, `$_ENV`, `$GLOBALS`, `$argv` | no variable is ever populated by the host; a request's data comes through `Core` classes | `E0211` |
| `$x = 1;` with no declaration | `int $x = 1;` or `var $x = 1;` — a local is declared once, with a type | `E0301` |
| `unset($x)` on a local or `unset($o->prop)` | none; `unset` removes an array entry only. Assign `null` where the type is `?T` | `E0234`, `E0413` |
| `class A { const X = 1; }`, `interface I { const X = 1; }` | `public const int X = 1;` — a constant writes its visibility like every member, and its type like every binding | `E0122`, `E0246` |

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
| `$a <> $b` | `!=` — the same comparison, and inequality has one spelling (ADR 0090 § 1) | `E0241` |

`<=>`, `**`, `??`, `??=`, `?:`, `?->`, `.=` and `instanceof` against a written class name all work
as in PHP.

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

# Classes

| PHP | Novis | Code |
|---|---|---|
| `function __construct(…)` | `public function constructor(…)` | `E0114` |
| `__toString()` | `implements Stringable` with `toString(): string` | `E0111` |
| `__get`, `__set`, `__isset`, `__unset` | a property hook (`public int $x { get => …; set (int $v) => …; }`) or `implements PropertyObserver`; an undeclared property is always an error | `E0111` |
| `__call`, `__callStatic` | none; declare the method | `E0111` |
| `__invoke` | a closure, `fn(…) => …` | `E0111` |
| `__destruct`, `__clone`, `__sleep`, `__wakeup`, `__serialize`, `__debugInfo`, `__set_state` | none: no destructors, no clone hook, no serialization hook | `E0111` |
| any name starting with `_` | no identifier starts with `_` | `E0111` |
| `function f()` inside a class (no visibility) | `public function f(): T` — every member writes `public`, `protected` or `private` | `E0122` |
| `trait T {}`, `use T;` inside a class | an interface method with a body for behaviour; `implements I by $field;` for state | `E0227` |
| `new class { … }` | a named class in the same file, or a closure where the class is one method — an anonymous class has no name for the static class table to hold | `E0244` |
| `readonly class A` | not a class modifier; `readonly` on a property parses | parse error `E0102` |
| `enum E: string { case A = "a"; }` | `enum E: int { A = 1 }` or `enum E { A, B }` — cases only, no `case` keyword, no methods or constants inside | `E0219`, `E0220` |
| `$n instanceof A` on a declared scalar, array or enum | the type already answers; declare the subject `mixed`, `object` or a class | `E0497` |
| `class order_line`, `function Total_Price()`, `const maxLines` | `PascalCase` class, `camelCase` member, `SCREAMING_SNAKE_CASE` constant — casing is a hard error | `E0110`–`E0113` |

Constructor promotion (`public function constructor(public int $x)`), `static::`, `parent::`,
`self::`, `abstract`, `final`, interfaces with constants and default method bodies, `clone`,
`new A` without parentheses and `A::class` all work.

# Closures and callables

| PHP | Novis | Code |
|---|---|---|
| `function (int $x) use ($k) { … }` | `fn(int $x): int => $x + $k;` — every outer variable read is captured by value, no `use` clause | `E0222`, `E0223` |
| `function (int $x) { … }` (no `use`) | `fn(int $x): int => { …; return …; }` — the block-body form | `E0222` |
| `fn($x) => $x` | `fn(int $x) => $x` — parameters declare a type; the return type may be inferred | `E0101` |
| `strlen(...)`, `A::f(...)`, `$o->m(...)` | write a closure: `fn(string $s): uint => Core\Str::length($s)` — first-class callable syntax type-checks but stops the compiler in this build | `E0320` for a free name |
| `call_user_func($f, 1)` | `$f(1)` — the answer of a call through `callable` is `mixed`, so `$f(1) as int` | — |

# Arrays and strings

| PHP | Novis | Code |
|---|---|---|
| `$a = [1, 2];`, `var $a = [1, 2];` | `array<int> $a = [1, 2];` — an array literal needs a declared target type | `E0414` |
| `foreach ([1, 2] as $n)` | bind the literal to a typed local first | `E0401` |
| `array(1, 2)` | accepted; `[1, 2]` is the usual spelling | — |
| `["a" => $x] = $arr;` keyed destructuring | index by key: `int $x = $arr["a"];` | parse error `E0101` |
| `print_r($v)`, `var_dump($v)`, `var_export($v)` | `Core\Debug::dump($v)` — writes to standard error, never to the output | `E0320` |
| `"$name"`, `"$a[0]"`, `"{$a[0]}"`, `"$o->x"`, `"{$o->x}"` | all interpolate as in PHP | — |
| `<<<EOT … EOT;`, `<<<'EOT' … EOT;` | both work, with PHP's interpolation rule for each | — |

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
