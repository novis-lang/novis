# Novis Specification — 00: Overview and Surface Syntax

- **Status:** Draft — the first spec document. It exists to fix *spelling* the ADRs deliberately left open;
  it does not restate any ADR's semantics, and where this document and an ADR ever disagree on wording, the
  ADR owns the semantics and this document owns the syntax (see [AGENTS.md](../../AGENTS.md), "every fact
  has exactly one home").
- **Scope:** file modes and inline HTML; `require` next to `eval` and `spawn script`, since the
  plan names confusing the last two as the predictable mistake; the concrete grammar for every declaration
  slot `rule:types/declaration` requires (typed locals, `foreach` bindings,
  destructuring, `type` aliases) and the final spelling of the conversion operator; the closed list of
  storage classes `rule:statements/static-is-a-member-modifier` decided, restated here only as a syntax
  table; the `bytes` literal question `rule:types/bytes` deferred here.
- **Out of scope, deliberately:** full expression/operator-precedence tables, the concurrency surface
  (`spawn`/`await`/`Channel`, M5), the HTTP/`Core` accessor surface (M7), and anything not named above. This
  file grows, and later spec files (`01-…`, `02-…`) are expected — it is not meant to be read as the whole
  language on its own.

> **In short:** a `.nvs` file opens code mode with `<?nvs` or the short echo tag `<?=`; everything else is
> inline HTML emitted verbatim, exactly like PHP. `require` shares
> everything with the calling frame and `spawn script` shares nothing but compiled code — the two are
> defined next to each other below so the difference cannot be missed the way
> `rule:security/isolate-shares-nothing` predicts it will be. Every declaration slot `rule:types/declaration`
> requires gets exactly one syntax: the type comes first, the same position PHP already uses for a parameter
> — `int $n = 0;`, `foreach ($rows as string $k => array<int> $row)`,
> `[int $a, string $b] = $pair;`, `type Row = array<string, int|string>;`. The conversion operator's
> provisional spelling in `rule:types/declaration` is hereby finalised: `expr as Type`, no other spelling, none planned.
> There is no `bytes` literal token: `"…" as bytes` covers valid-UTF-8 payloads for free, and
> `Core\Encoding::fromHex()`/`::fromBase64()` cover binary constants that are not valid UTF-8 — a lexer with
> one fewer token shape than a dedicated `b"…"` prefix would have needed.

## 1. File modes and inline HTML

A source file is lexed in one of two modes, exactly as PHP is. The lexer reads content and never a file
name, so the extension decides nothing here — `.nvs` is the convention, and a PHP file renamed to it is
still refused at its `<?php` tag:

- **HTML mode**, the default at the start of a file and after a closing `?>`. Every byte is emitted verbatim
  as output, with no escaping, until the lexer sees an opening tag.
- **Code mode**, entered by one of two opening spellings and left by `?>`:

  | opens | leaves | meaning |
  |---|---|---|
  | `<?nvs` | `?>` | ordinary code mode — the only code-mode open tag Novis keeps |
  | `<?=` `expr` | `?>` | short-echo: exactly `<?nvs echo expr; ?>`, one expression, `;` optional before `?>` |

  `<?php` is diagnosed rather than accepted: `rule:statements/nvs-is-the-only-open-tag`
  withdraws its earlier acceptance as a second spelling of `<?nvs`, because a PHP file does not run
  unported whatever its tag says.

  A `?>` immediately followed by a single newline consumes that newline (PHP's rule, kept so a template line
  ending in `?>` does not emit a blank line). There is no closing-tag omission rule beyond that: an unclosed
  `<?nvs` block simply runs to end of file, which is legal and is how a pure-code `.nvs` file with no inline
  HTML is written.

There is no dual short-open-tag ambiguity to resolve (PHP's long-deprecated bare `<?`): Novis never had it, so
there is nothing to accept or reject.

## 2. Running another file: `require`, `eval`, and `spawn script`

Two PHP-shaped ways to bring in code plus one Novis-only addition, and they isolate three different amounts.
Defined here side by side because `rule:security/isolate-shares-nothing` names exactly this
confusion as the mistake worth heading off. `rule:statements/require-is-the-only-inclusion-construct`
collapses PHP's four same-frame inclusion keywords to this one: `include`, `include_once`, and
`require_once` all parse (so the diagnostic can name the replacement) and are then rejected.

| construct | isolation | resolution | status |
|---|---|---|---|
| `require 'path.nvs';` | **none** — same frame's globals, same statics, same output, same heap | statically resolved where the path is a literal (M2); a dynamic path falls back to a runtime resolve | kept, PHP semantics — throws on a missing/unparseable file, and runs every time control reaches it |
| `eval($source)` | n/a — there is no such construct | n/a | **rejected**, no diagnostic-with-replacement needed beyond *there is no `eval`*: a string has no stable identity, no cache key, and no path a `script.spawn` grant could name. `rule:security/no-eval` holds the full rejection and the four analyses `eval` would make unsound at once; see also [ADR 0006](../decisions/0006.md), *Alternatives rejected* |
| `spawn script 'path.nvs' with(…)` | **full** — fresh arena, fresh globals/statics, own config overlay, sharing only immutable compiled code | the path is an arbitrary `string` expression, canonicalised and prefix-checked against `script.spawn`'s granted roots at run time (M6); the operand may instead be a static method — `Class::method(...)`, called with `args:` bound to its parameters by name — decided at the spawn site (`rule:security/isolate-shares-nothing`) | new construct, grammar fixed below |

The rule of thumb the diagnostics should teach: **`require` runs code in this frame; `spawn script` runs a
file as if it were its own request.** A "why can't the required/spawned code see my variable" question
should get a different answer depending on which of the two produced it — `require` never hides a variable
declared before it (there is nothing to hide), while a variable invisible inside a `spawn script` child is
expected and the diagnostic at the child's use site should name the isolate boundary as the reason, not
report a plain undefined-variable error.

### Reaching a declaration without naming its file: `autoload`

`require` names a file. `autoload` names a *rule* for finding files, so ordinary code never names one at
all — `rule:programs/no-runtime-autoload` owns the semantics (including
why there is no manifest file and no runtime loader), this owns the grammar.

```
autoload-decl := "autoload" prefix "from" path-list ";"
               | "autoload" "discover" glob ";"

prefix        := string-literal   // a namespace prefix, no trailing separator: 'Acme\Legacy'
path-list     := string-literal ("," string-literal)*
glob          := string-literal   // exactly one "*", occupying a whole path segment
```

```php
autoload 'Framework' from './';
autoload 'Acme\Legacy' from '../vendor/acme/lib', '../vendor/acme/compat';
autoload discover '../../*/src';
```

Every path is a plain string literal — no interpolation, no concatenation, the same restriction `require`'s
static resolution carries — and resolves **relative to the directory of the file the declaration appears
in**, never relative to the entry point. The statement is valid only at a file's top level, and only in a
file reachable by `require` from the entry point; the effective map is the union of every such declaration.

`autoload` is a reserved word. `discover` is **contextual** — it means the second form only directly after
`autoload`, and is an ordinary identifier (a method name, a class name) everywhere else.

### `spawn script` grammar

```
spawn-script-expr := 'spawn' 'script' expr with-clause?
with-clause        := 'with' '(' spawn-option (',' spawn-option)* ','? ')'
spawn-option        := 'args'   ':' expr
                      | 'limits' ':' expr
                      | 'grants' ':' expr
                      | 'output' ':' expr
                      | 'on'     ':' expr
await-expr          := 'await' unary-expr
```

`spawn-script-expr` is an expression, not a statement, so it can appear anywhere an expression can (assigned,
passed, awaited inline). The operand `expr` is one of two things. A path has static type `string` and is
evaluated once, at the spawn site, before the child isolate is created. A static method — a
`Class::method(...)` reference — names a function in an already-compiled unit as the entry instead, and
the isolate calls it with `args:`'s entries as named arguments; the choice is made syntactically at the
spawn site, and an `fn` literal or a `callable`-typed variable is refused there with a diagnostic naming
the method form, all per `rule:security/isolate-shares-nothing`. `with(…)` reuses PHP's existing named-argument grammar verbatim —
no new call-argument syntax was needed for it. Every key in *spawn-option* is optional; `spawn script
'jobs/report.nvs';` with no `with(…)` clause at all is legal and spawns with inherited grants, no argument,
and the parent's remaining budget.

The expression's static type is `Core\Script\Handle`, a registered class with no members at all — `await` is
the only thing a program may do with one — and `await` answers with a `ScriptResult` **shape** rather than
with a class. Both are the concurrency type surface M5 defines rather than this document's:
`crates/nvs-types/src/expr/isolate.rs`'s module doc is the one home of the field set and of why the two
halves take opposite mechanisms, and `crates/nvs-stdlib/src/script.rs` is the handle's row. This section
fixes only the `spawn script … with(…)` token sequence, so that the M1 parser has a grammar to implement
without waiting on M5's design.

`await`'s operand is a *unary* expression, so the postfix chain binds tighter than the keyword does and
`await $h->handle` awaits the property. Both `spawn`/`script`/`with` and `await` are **contextual**: they
lex as ordinary identifiers and are read as this grammar only where one is followed by what the production
needs — `script` after `spawn`, an operand after `await` — so a program that already uses either spelling
as a constant, a function or a method name still parses. The one reading this claims from such a program is
`await($x)`, which is the operator over a parenthesised operand and not a call to a function named `await`.

## 3. The declaration-slot grammar (`rule:types/declaration`'s spelling)

[ADR 0007 § 3](../decisions/0007.md) already normatively fixes the *type expression* grammar
(`type := union := …`, `array<T>`, unions, intersections, `mixed`, and so on) — this document does not
repeat it and adds no new production to it. What `rule:types/declaration` left to the spec is the *statement*-level grammar
around each new declaration slot: where exactly a `type` sits relative to the `$` sigil, and how a
destructuring target is written. One rule threads all of them: **the type comes first, in the same position
PHP already uses for a parameter** — there is exactly one type-then-sigil shape in the whole language,
never a colon-suffixed alternative, so a reader who has learned `function f(int $n)` has learned the shape
for every other slot too.

### 3.1 Typed local declaration

```
local-decl-stmt := type '$' identifier ('=' expr)? ';'
```

```php
int    $n     = 0;
array<uint> $ids;
User   $owner = User::find($id);
```

A local is declared exactly once, at this statement. Every later `$n = 5;` is a plain assignment-expression
statement, with no type prefix — writing one again is a re-declaration, which
[ADR 0007 § 1](../decisions/0007.md) already makes a diagnostic naming the first declaration.

### 3.2 `foreach` with typed bindings

```
foreach-stmt   := 'foreach' '(' expr 'as' foreach-target ')' block
foreach-target := type '$' identifier
                 | type '$' identifier '=>' type '&'? '$' identifier
```

```php
foreach ($names as string $name) { … }
foreach ($rows  as string $k => array<int> $row) { … }
foreach ($items as string $k => inout int $v) { $v += 1; }   // inout binds the value only, as in PHP
```

Both the key and the value binding are typed — there is no untyped `foreach ($rows as $row)` left, the same
mandatory-declaration rule as everywhere else in `rule:types/declaration`. A reference marker (`&`) may only appear on the
value binding, never the key, matching PHP's own restriction.

**The grammar wrinkle `rule:types/conversion` calls out by name:** inside a `foreach` header, `as` is `foreach`'s own
keyword, not the conversion operator, so converting the *subject* — not a binding — needs parentheses:

```php
foreach (($m as array<int>) as int $v) { … }     // convert $m to array<int>, then bind each element as int
```

This is the one case M1's snapshot tests must pin down explicitly, per the plan's M1 verification section.

### 3.3 Destructuring

```
destructure-target := '[' destructure-elem (',' destructure-elem)* ']'
destructure-elem    := (( string-literal | expr ) '=>')? destructure-target
                      | (( string-literal | expr ) '=>')? type '&'? '$' identifier
                      |                                                        // an empty slot, list-style skip
destructure-stmt    := destructure-target '=' expr ';'
```

```php
[int $a, string $b]              = $pair;
[, int $second]                  = $triple;                 // skip the first element
['id' => uint $id, 'name' => string $name] = $row;
[[int $x, int $y], string $label] = $point;                  // nested destructuring, typed at every leaf
```

`list($a, $b) = $expr;` does not parse: `[...]` is the only destructuring spelling, and `list(...)` is
diagnosed at parse time naming it (`rule:expressions/bracket-destructuring`). Every
leaf names its type where it is bound, at any nesting depth, mirroring how `array<array<uint>>` nests in the
type grammar itself; there is no untyped form.

### 3.4 The conversion operator: `as`, finally

[ADR 0007 § 2](../decisions/0007.md) opens with "provisional spelling `expr as T`." This
document fixes that spelling as **final**: `as` is the conversion operator, with no alternate spelling, and
none is planned. Its precedence and throwing behaviour are exactly as `rule:types/conversion` already states and are
not repeated here.

The target may be nullable, and `expr as ?T` is **not** a second spelling — it is the same operator over a
type the grammar already accepted, yielding `null` where the throwing form would throw
(`rule:expressions/nullable-conversion`, which owns which conversions admit it).

### 3.5 `type` aliases

```
type-alias-decl := 'type' ClassName '=' type ';'
```

Sits at file/namespace scope, alongside `use` and `namespace` — never inside a class body, per
[ADR 0015 § 5](../decisions/0015.md). `type` on its own is otherwise an ordinary reserved word in
this one declaration position; it is not a general statement keyword. The restriction that `TypeExpr` may
not be a single bare class/interface/enum atom ([ADR 0015 § 6](../decisions/0015.md)) is a
resolution-time check (M2), not a parse-time one — the grammar above parses `type Id = SomeClass;` exactly
like any other alias declaration, and M2's resolver is where it becomes a diagnostic.

## 4. The scoping surface (`rule:statements/static-is-a-member-modifier`'s spelling only)

[ADR 0008 § 2](../decisions/0008.md) is the closed, exhaustive list of where state may outlive a
call, and this document adds no storage class to it and repeats none of its reasoning. What belongs here is
only the syntax for each row, gathered in one place since it is otherwise scattered across `rule:types/declaration` and `rule:statements/static-is-a-member-modifier`'s own examples:

| storage | syntax |
|---|---|
| local variable, parameter | § 3.1 above; `function f(int $n) { … }` |
| class static property | `private static int $calls = 0;` |
| class constant | `public const int MAX = 10;` |
| object property | `public readonly uint $id;` |
| top-level script variable | § 3.1's grammar, written at file scope instead of inside a function |

`static` never appears as a declaration keyword outside a class member; there is no function-scope
`static int $x`, no `static fn`, and no `global` — each rejected with the diagnostic `rule:statements/no-function-static-and-no-global` already
names. This document fixes no new syntax for any of the three, because there is no replacement syntax to
fix: each rejection's replacement is one of the five rows above, already covered.

## 5. `bytes`: no dedicated literal

`rule:types/bytes` leaves `bytes` literal syntax to this document. Decision: **there
is no `bytes` literal token.** The lexer needs no `b"…"`-shaped production, and M1's scope is smaller for it.

- For the common case — a byte sequence that happens to be valid UTF-8, which is most binary-ish constants a
  program writes by hand (magic strings, protocol markers made of ASCII) — a plain string literal converts
  for free: `"NVS1" as bytes`. [ADR 0009 § 3](../decisions/0009.md) already makes `string as
  bytes` total and free, so this is not a new conversion rule, only its first literal-adjacent use.
- For a byte sequence that is **not** valid UTF-8 — a raw binary constant, a fixed hash or key material
  written inline — the spelling is a `Core\Encoding` decoder, following
  `rule:classes/no-free-functions-or-constants`'s "every callable is a class member"
  rule exactly as every other domain class does: `Core\Encoding::fromHex('deadbeef')`,
  `Core\Encoding::fromBase64('...')`. That class, not `Core\Bytes`, because
  [01-core-library § 7](01-core-library.md) sites every `bytes`↔`string` conversion there, and each of
  these is one. Both join the domain-class roster `rule:classes/no-free-functions-or-constants`'s summary names as examples, not as a closed list; building
  them is ordinary M4S stdlib work, not part of this spec.

This resolves one item in [ADR 0009 *Revisiting*](../decisions/0009.md), and only that one: it is
a decision about literal syntax, not about `string`'s default length/indexing granularity, which § 2 of that
ADR owns.

## Revisiting

- This document does not yet fix operator precedence beyond the two points `rule:types/declaration` already pins (`as`
  binds tighter than any binary operator; `foreach`'s `as` versus the conversion operator). A full
  precedence table belongs in a later spec file once the parser needs one written down rather than inferred
  from the grammar productions above.
- `spawn`/`await`/`Channel`/`spawn worker` and the rest of the concurrency surface are named here only where
  `spawn script`'s grammar needs to distinguish itself from them; M5 is where they get their own spec
  section.
- If `.phpt`-corpus parsing (M1's verification) turns up a real PHP file using a short open tag (bare `<?`),
  that is a decision to make then, not a gap in this document — Novis never had one to begin with, so adding
  one would be a new grammar choice, not a restatement of an existing one.
