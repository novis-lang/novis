---
id: programs
title: Programs, files and names
summary: what a `.nvs` file is, how it runs, how names are spelled and resolved, and how one file reaches another
keywords: <?nvs, <?=, ?>, inline HTML, shebang, nvs run, echo, print, comments, exit, namespace, use, require, autoload, discover, class name, casing, constructor, visibility, top-level statements, main
---

# A program is a file of top-level statements

A Novis program is a `.nvs` file. It is run directly — `nvs run hello.nvs` — with no build step
and no `main`: the statements at the top level of the file are the program, executed in order.

```nvs
<?nvs
echo "Hello, World!", "\n";
```
```output
Hello, World!
```

`echo` takes any number of comma-separated expressions and writes them, unseparated, to standard
output; `print expr;` writes one. Nothing is appended for you — `"\n"` is the newline.

<!-- primer -->
# A complete program, annotated

The shapes a program reaches for in its first ten lines, in one file that runs. Each is specified
in the chapter that owns it — types, statements, expressions — and named in the comment beside it.

```nvs
<?nvs
// A local declares its type once, and no binding ever changes type.
array<string> $amounts = ["3", "11", "7"];

// A member's optional knobs are one trailing object literal, `{key: value}`. They are not named
// arguments and there are no flag parameters: this signature spells the bag `{order?: Core\Order}`.
array<string> $ordered = Core\Arr::sort($amounts, {order: Core\Order::Asc});

int $total = 0;
// Every `foreach` binding declares its type; an untyped `as $each` does not parse.
foreach ($ordered as string $each) {
    // `as` is the one conversion operator: no casts, and a string that is not an integer throws.
    $total = $total + ($each as int);
}

// Every built-in is a class member; the language has no free functions.
echo Core\Json::encode($ordered), " ", $total, "\n";
```
```output
["11","3","7"] 21
```

Nothing in it reaches outside the program. A member that reads a file, opens a connection or starts
another process is denied at the call until `nvs.toml` grants the capability it names.

# Code mode and HTML mode

A file starts in **HTML mode**: every byte is copied to the output verbatim until an opening tag.
`<?nvs` enters **code mode**; `?>` leaves it again. A pure-code file is simply one that opens with
`<?nvs` and never closes it — the tag runs to the end of the file.

```nvs
<html><body>
<?nvs
string $who = "world";
?>
<p>Hello, <?= $who ?>!</p>
</body></html>
```
```output
<html><body>
<p>Hello, world!</p>
</body></html>
```

- `<?= expr ?>` is short for `<?nvs echo expr; ?>` — one expression, written into the output where
  the tag stands. The `;` before `?>` is optional in both forms.
- A `?>` followed directly by one newline swallows that newline, so a template line ending in a
  closing tag emits no blank line.
- `<?php` is refused with a diagnostic naming `<?nvs`, and there is no short open tag `<?`, so a
  `<?xml` declaration stays text.
- A file whose first two bytes are `#!` is in code mode from line 2 with no opening tag: line 1 is
  trivia rather than output, and an `<?nvs` before the first `?>` is `E0009`
  (`rule:tooling/shebang-opens-code-mode`).

Output inside a request or on a terminal goes through a *sink*, and the terminal sink substitutes
control bytes visibly rather than passing them through; that is covered with qualifiers in the
types chapter.

Inside an HTTP request the sink escapes every `string` it is given and writes a `Core\Html\Markup`
raw, so `<?= $title ?>` cannot emit a tag. A page or a fragment of one built as a value is an
``html`…` `` literal — trusted text around `{$…}` holes that are escaped — and a method that
returns one is how a page is composed from parts. The literal is the types chapter's `Markup: the
html template literal`.

Code mode and HTML mode alternate freely, and a brace block may span them — the ordinary way to
render a loop or a condition around raw HTML:

```nvs
<?nvs
array<string> $products = ["pen", "ink", "paper"];
?>
<ul>
<?nvs foreach ($products as string $name) { ?>
  <li><?= $name ?></li>
<?nvs } ?>
</ul>
```
```output
<ul>
  <li>pen</li>
  <li>ink</li>
  <li>paper</li>
</ul>
```

# Comments

```nvs
<?nvs
// a line comment
# also a line comment
/* a block
   comment */
echo "ok", "\n"; // trailing
```
```output
ok
```

`#[` — with the bracket — opens an attribute (the attributes chapter), not a comment.

# Names and casing

Every identifier's spelling is checked by the compiler, and a wrong case is a compile error with
no way to suppress it:

| Thing | Spelling | Example |
|---|---|---|
| class, interface, enum, type alias, namespace segment | `PascalCase` | `class OrderLine`, `namespace App\Billing` |
| method, property, parameter, local variable | `camelCase` | `function totalPrice()`, `$lineCount` |
| class constant, enum case | constant: `SCREAMING_SNAKE_CASE`; case: `PascalCase` | `const int MAX_LINES = 10;`, `Rank::Gold` |
| the constructor | exactly `constructor` | `public function constructor(int $n) { … }` |

- No identifier may start with `_`. PHP's `__construct`, `__toString` and every other magic method
  do not exist; the constructor is `constructor` and stringification is the `Stringable` interface
  (the classes chapter).
- Every class member — property, method, constant — writes its visibility (`public`, `protected`,
  `private`). There is no implicit `public`.
- Names are case-sensitive everywhere, including the file names `autoload` resolves.

```nvs error
<?nvs
class order_line {
    public int $n = 1;
}
```

# Namespaces and `use`

A file may open with one `namespace` declaration, and any number of `use` imports after it. Both
are file-level: neither may appear inside a class or a function body.

```nvs file=lib/Text/Greeter.nvs
<?nvs
namespace App\Text;

class Greeter {
    public static function greet(string $name): string {
        return "Hello, " . $name . "!";
    }
}
```
```nvs
<?nvs
namespace App;

use App\Text\Greeter;
use Core\Str;

autoload 'App' from './lib';

echo Greeter::greet("Ada"), "\n";
echo Str::upper("shout"), "\n";
echo Core\Str::lower("QUIET"), "\n";
```
```output
Hello, Ada!
SHOUT
quiet
```

How a written name resolves:

- **A name containing `\` is absolute.** `Core\Str` always means the class `Core\Str`, whatever the
  current namespace or imports; a name never resolves relative to the enclosing namespace once it
  has a separator. A *leading* `\` does not parse — there is nothing for it to disambiguate.
- **A name without `\`** is looked up in the file's `use` imports first, then in the file's own
  namespace, and nowhere else. There is no fallback to the global namespace: inside `namespace App;`,
  an unimported `Helper` means `App\Helper`, never a `Helper` declared outside any namespace.
- `use A\B\C;` imports one name, under its own short name `C`. There is no renaming: `use A\B\C as D;`
  is refused, and so is PHP's group form `use A\{B, C};` — write one `use` per name.
- `Core` is a reserved namespace: `namespace Core;` is refused, so nothing a program declares lives
  under it. A global class may still be named `Str` — only the `Core\` prefix is reserved.
- A `class`, `interface`, `enum` or `type` alias is declared at file scope, or not at all.

# `require`: run another file in this frame

`require 'path.nvs'` reads, compiles and runs another file where the `require` is written — same
program, same statics, same output, no isolation of any kind — every time control reaches it, and it
is an *expression* whose value is whatever the required file `return`s at its top level.

```nvs file=config.nvs
<?nvs
return ["host" => "example.test", "port" => "8080"];
```
```nvs
<?nvs
array<string> $config = (require 'config.nvs') as array<string>;
echo $config["host"], ":", $config["port"], "\n";
```
```output
example.test:8080
```

- The path is relative to the requiring file's directory. A literal path is resolved while
  compiling, so a missing file is a compile-time error; a computed path is resolved at run time and
  throws if it cannot be read.
- The value of a `require` expression is `mixed` — convert it with `as` to the type the file returns.
- **Declarations cross and variables do not.** Every class, interface, enum and `type` alias either
  file declares is visible to the other as if it had been pasted in. A local does not cross in either
  direction: each file's top-level body is its own frame and is checked on its own, so the required
  file's `$x` is not the caller's. Hand a value across as the `return` value or through a static.
- `include`, `include_once` and `require_once` do not exist. Each parses only so the diagnostic can
  name `require`.
- A file that must run once — a declarations file — is reached through `autoload` instead, which
  never runs a file twice.

# `autoload`: find a class by its namespace

`autoload` declares a rule that maps a namespace prefix to a directory, so ordinary code names a
class and never a file. `App\Text\Greeter` under `autoload 'App' from './lib';` is read from
`./lib/Text/Greeter.nvs` — the prefix is stripped, the remaining segments become directories, and
the last becomes the file name.

```nvs skip
autoload 'App' from './src';                       // one prefix, one root
autoload 'Acme\Legacy' from '../vendor/acme/lib', '../vendor/acme/compat';   // several roots, searched in order
autoload discover '../packages/*/src';             // every directory the glob matches is a root for the namespace named by the `*` segment
```

- Every path is a plain string literal, relative to the directory of the file holding the
  declaration. The statement is valid only at a file's top level.
- The map is built while compiling, from every `autoload` in every file the program reaches;
  there is no runtime loader and no registration function. A class that cannot be found is a
  compile error naming the roots that were searched.
- The same map answers `Core\Program::implementing<I>()` — every non-abstract class implementing
  an interface, found through the autoload roots even when nothing names it.

# Ending a program

A program ends when its last top-level statement has run, with exit status 0. `exit;` ends it
early with status 0, `exit(3);` with the status given, and `exit("message");` prints the message
and exits with status 0 — the same three forms PHP's `exit` has. A status is one byte: the low
eight bits are what the process reports on every platform, so `exit(300)` exits 44 and `exit(-1)`
exits 255, as in PHP. `die` is not a second spelling — it parses, and only so the compiler can
point at `exit` (`E0228`, `rule:statements/exit-is-the-only-termination-keyword`). An uncaught
throw ends the program with status 1 and a backtrace on standard error (the errors chapter).

```nvs exit=3
<?nvs
echo "stopping", "\n";
exit(3);
echo "not reached", "\n";
```
```output
stopping
```
