---
id: php-differences
title: "Coming from PHP: how Novis is different, and how to port a program"
summary: how Novis is different from PHP, what Novis can do that PHP cannot, and how to port a program by writing it again; there is no converter and no list of PHP names
keywords: PHP, coming from PHP, switch from PHP, differences, port, porting, rewrite, migration, converter, AI agent, coding agent, nvs agent, PHPUnit, PHPStan, Psalm, PHP CS Fixer, PHP_CodeSniffer, Composer
---

This page is for somebody who already knows PHP. Novis looks like PHP, but it is a different
language. This page explains how Novis is different, what Novis can do that PHP cannot, and how to
port a program.

PHP code does not run in Novis. There is no converter. There is also no list that gives the Novis
method for each PHP function. A PHP program and a Novis program are built from different parts:
Novis has no traits, no magic methods and no global functions. A tool cannot choose the new design
for your program, so you write it yourself, or with the help of an AI coding agent.

# The short list

Ten ideas explain most of the differences:

- **Every variable, parameter and property has a type**, and you write it once. A value never
  changes its type. `mixed` is the type for a value that can have more than one type.
- **Every function is a method of a class, and every constant is a constant of a class.** This is
  true for the built-in ones too. Novis has no global variables, so `$_GET`, `$GLOBALS` and
  `global` do not exist.
- **The `Core` classes are the standard library.** All their methods use the same order of
  arguments and accept named arguments. A method that fails throws an error. It does not return
  `false`.
- **There is one equality operator.** `==` never converts its operands, and `===` does not exist.
  Comparing two values of unrelated types does not compile.
- **A `string` is UTF-8 text** and counts graphemes (what a person counts as one character). Binary
  data has its own type, `bytes`.
- **An array keeps its insertion order, and its keys are always strings.** It declares the type of
  its elements. Assigning an array to a second variable gives a copy.
- **Novis has no `eval`, no references and no magic methods.** The constructor is named
  `constructor`. A parameter marked `inout` is the only way a method can change a variable of its
  caller.
- **Concurrency is built in.** `Core\Task` and channels run work at the same time. `spawn script`
  starts a separate script that shares no memory with yours.
- **Security is part of the program.** A program needs a permission in `nvs.toml` to read a file or
  to open a network connection, for example `fs.read` or `net.connect`. `tainted` and `secret` are
  part of a type, and the compiler checks where those values go.
- **Tests are part of the language.** You write `#[Test]` methods and run them with `nvs test`. See
  *The tools you do not install* below.

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

## What Novis can do that PHP cannot

Three things in Novis are part of the language, and a library cannot add them to PHP:

- **SQL injection and leaked secrets are compile errors.** Text from a request is `tainted`. A
  password or a key is `secret`. The program does not compile if such a value is used in an unsafe
  place, for example tainted text in an SQL query.
- **Every request runs in its own isolate.** An isolate is a separate part of one server process,
  with its own memory. Each request, job and script has its own limit for memory, CPU and time, and
  one request cannot read another request's data.
- **Any function can wait.** A method that reads a file or calls a server waits without blocking the
  other requests. There is no `async` keyword, so you never write a second version of a function.

# Porting a program

To port a PHP program, read it to learn what it does. Then write that again with the parts Novis
has. Do not translate it line by line. A trait, a magic method or a global helper function has no
direct form in Novis, and you choose the new design.

1. Write down what the program does: its pages, its jobs, and the data it reads and writes.
2. Write the types first: the classes, their typed properties and the enums.
3. Write each part again with the `Core` classes. `nvs agent find` with the name of a task finds
   the method for it.
4. Run `nvs check` after each part. It shows every line that still needs a change.
5. Write tests as `#[Test]` methods, and run them with `nvs test`.

For a large program, you can give this work to an AI coding agent. `nvs agent init` writes a file
that tells the agent about `nvs agent`. The agent then reads `nvs agent primer`, and it looks up
each method with `nvs agent find` and `nvs agent show`. See [agents](#tools-agents).

This is a small cart program written in Novis. The function `total` is a method of the class
`Cart`, and its parameter has the type `array<int>`. `as int` converts the text `"7"` to the number
`7`, and `&&` is the way to write "and":

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

# The tools you do not install

A PHP project usually installs a set of development tools with Composer. Novis has their work built
into the toolchain or into the language itself:

- **PHPUnit** → `#[Test]` methods, `Core\Test` assertions and `nvs test` are the framework — data
  rows, fixtures, skip-with-reason, retries, JUnit and JSON output. See [testing](#lang-testing).
- **PHPStan / Psalm** → `nvs check` is the compiler, and it already checks what their strictest
  levels check: every variable typed, every method resolved at compile time, every path returning,
  every property initialized — plus `tainted`/`secret` flow, which is Psalm's taint mode as a type
  rule. There are no levels and no baseline file, because there is no untyped code to bridge.
- **PHP CS Fixer / PHP_CodeSniffer** → the style rules that catch bugs are compile errors here —
  identifier casing, a written visibility on every member, one way to write each construct — with
  no suppression and nothing to configure.

A test is a method with `#[Test]` in any class. `nvs test` runs this file and reports three tests:
one for each `#[TestWith]` row, and one that is skipped with its reason.

```nvs test
<?nvs
use Core\Test;
use Core\Test\TestWith;

final class Price {
    public static function withTax(int $cents, int $percent): int {
        return $cents + Core\Math::intDiv($cents * $percent, 100);
    }
}

final class PriceTest {
    #[TestWith(cents: 1000, percent: 20, want: 1200)]
    #[TestWith(cents: 999, percent: 10, want: 1098)]
    #[Test]
    public function taxIsAdded(int $cents, int $percent, int $want): void {
        Test::assertSame(Price::withTax($cents, $percent), $want);
    }

    #[Test(skip: "refunds are not written yet")]
    public function aRefundGivesTheTaxBack(): void {
        Test::assertSame(Price::withTax(-1000, 20), -1200);
    }
}
```
```output
  PriceTest
    ✓ taxIsAdded#0
    ✓ taxIsAdded#1
    - aRefundGivesTheTaxBack
      skipped: refunds are not written yet
  0 failed, 2 passed, 1 skipped, 0 flaky in
```

`nvs check` reports a method that does not return a value on every path. No analyser is set up
for it:

```nvs error
<?nvs
class Stock {
    public static function label(int $count): string {
        if ($count > 0) {
            return "in stock";
        }
    }
}

echo Stock::label(3), "\n";
```
```output
E0739
```
