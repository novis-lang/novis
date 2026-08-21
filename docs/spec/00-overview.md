# MWL Specification — 00: Overview and Surface Syntax

- **Status:** Draft — the first spec document. It exists to fix *spelling* the ADRs deliberately left open;
  it does not restate any ADR's semantics, and where this document and an ADR ever disagree on wording, the
  ADR owns the semantics and this document owns the syntax (see [CLAUDE.md](../../CLAUDE.md), "every fact
  has exactly one home").
- **Scope:** file modes and inline HTML; `require` next to `eval` and `spawn script`, since the
  plan names confusing the last two as the predictable mistake; the concrete grammar for every declaration
  slot [ADR 0007](../adr/0007-explicit-type-system.md) requires (typed locals, `foreach` bindings,
  destructuring, `type` aliases) and the final spelling of the conversion operator; the closed list of
  storage classes [ADR 0008](../adr/0008-static-and-global.md) decided, restated here only as a syntax
  table; the `bytes` literal question [ADR 0009](../adr/0009-string-and-bytes.md) deferred here.
- **Out of scope, deliberately:** full expression/operator-precedence tables, the concurrency surface
  (`spawn`/`await`/`Channel`, M5), the HTTP/`Core` accessor surface (M7), and anything not named above. This
  file grows, and later spec files (`01-…`, `02-…`) are expected — it is not meant to be read as the whole
  language on its own.

> **In short:** a `.mwl` file opens code mode with `<?mwl`, `<?php` (accepted, same grammar), or the short
> echo tag `<?=`; everything else is inline HTML emitted verbatim, exactly like PHP. `require` shares
> everything with the calling frame and `spawn script` shares nothing but compiled code — the two are
> defined next to each other below so the difference cannot be missed the way
> [ADR 0006](../adr/0006-isolated-script-execution.md) predicts it will be. Every declaration slot ADR 0007
> requires gets exactly one syntax: the type comes first, the same position PHP already uses for a parameter
> — `int $n = 0;`, `foreach ($rows as string $k => array<int> $row)`,
> `[int $a, string $b] = $pair;`, `type Row = array<string, int|string>;`. The conversion operator's
> provisional spelling in ADR 0007 is hereby finalised: `expr as Type`, no other spelling, none planned.
> There is no `bytes` literal token: `"…" as bytes` covers valid-UTF-8 payloads for free, and
> `Core\Bytes::fromHex()`/`::fromBase64()` cover binary constants that are not valid UTF-8 — a lexer with one
> fewer token shape than a dedicated `b"…"` prefix would have needed.

## 1. File modes and inline HTML

A `.mwl` file (and a `.php` file, parsed under the same grammar — see the plan's M1 verification) is lexed
in one of two modes, exactly as PHP is:

- **HTML mode**, the default at the start of a file and after a closing `?>`. Every byte is emitted verbatim
  as output, with no escaping, until the lexer sees an opening tag.
- **Code mode**, entered by one of three opening spellings and left by `?>`:

  | opens | leaves | meaning |
  |---|---|---|
  | `<?mwl` | `?>` | ordinary code mode |
  | `<?php` | `?>` | accepted as a second spelling of `<?mwl`, identical parse — the pragmatic-superset
    promise applies to the tag itself, not only to what is inside it |
  | `<?=` `expr` | `?>` | short-echo: exactly `<?mwl echo expr; ?>`, one expression, `;` optional before `?>` |

  A `?>` immediately followed by a single newline consumes that newline (PHP's rule, kept so a template line
  ending in `?>` does not emit a blank line). There is no closing-tag omission rule beyond that: an unclosed
  `<?mwl`/`<?php` block simply runs to end of file, which is legal and is how a pure-code `.mwl` file with no
  inline HTML is written.

There is no dual short-open-tag ambiguity to resolve (PHP's long-deprecated bare `<?`): MWL never had it, so
there is nothing to accept or reject.

## 2. Running another file: `require`, `eval`, and `spawn script`

Two PHP-shaped ways to bring in code plus one MWL-only addition, and they isolate three different amounts.
Defined here side by side because [ADR 0006](../adr/0006-isolated-script-execution.md) names exactly this
confusion as the mistake worth heading off. [ADR 0021](../adr/0021-single-file-inclusion-construct.md)
collapses PHP's four same-frame inclusion keywords to this one: `include`, `include_once`, and
`require_once` all parse (so the diagnostic can name the replacement) and are then rejected.

| construct | isolation | resolution | status |
|---|---|---|---|
| `require 'path.mwl';` | **none** — same frame's globals, same statics, same output, same heap | statically resolved where the path is a literal (M2); a dynamic path falls back to a runtime resolve | kept, PHP semantics — throws on a missing/unparseable file, and runs every time control reaches it |
| `eval($source)` | n/a — there is no such construct | n/a | **rejected**, no diagnostic-with-replacement needed beyond *there is no `eval`*: a string has no stable identity, no cache key, and no path a `script.spawn` grant could name (see [ADR 0006](../adr/0006-isolated-script-execution.md), *Alternatives rejected*) |
| `spawn script 'path.mwl' with(…)` | **full** — fresh arena, fresh globals/statics, own config overlay, sharing only immutable compiled code | the path is an arbitrary `string` expression, canonicalised and prefix-checked against `script.spawn`'s granted roots at run time (M6) | new construct, grammar fixed below |

The rule of thumb the diagnostics should teach: **`require` runs code in this frame; `spawn script` runs a
file as if it were its own request.** A "why can't the required/spawned code see my variable" question
should get a different answer depending on which of the two produced it — `require` never hides a variable
declared before it (there is nothing to hide), while a variable invisible inside a `spawn script` child is
expected and the diagnostic at the child's use site should name the isolate boundary as the reason, not
report a plain undefined-variable error.

### `spawn script` grammar

```
spawn-script-expr := 'spawn' 'script' expr with-clause?
with-clause        := 'with' '(' spawn-option (',' spawn-option)* ','? ')'
spawn-option        := 'args'   ':' expr
                      | 'limits' ':' expr
                      | 'grants' ':' expr
                      | 'output' ':' expr
                      | 'on'     ':' expr
```

`spawn-script-expr` is an expression, not a statement, so it can appear anywhere an expression can (assigned,
passed, awaited inline). The path `expr` must have static type `string`; it is evaluated once, at the spawn
site, before the child isolate is created. `with(…)` reuses PHP's existing named-argument grammar verbatim —
no new call-argument syntax was needed for it. Every key in *spawn-option* is optional; `spawn script
'jobs/report.mwl';` with no `with(…)` clause at all is legal and spawns with inherited grants, no argument,
and the parent's remaining budget.

The expression's static type — an awaitable handle that a subsequent `await` turns into a `ScriptResult` —
belongs to the concurrency type surface M5 defines, not to this document; this section fixes only the
`spawn script … with(…)` token sequence, so that the M1 parser has a grammar to implement without waiting on
M5's design.

## 3. The declaration-slot grammar (ADR 0007's spelling)

[ADR 0007 § 3](../adr/0007-explicit-type-system.md) already normatively fixes the *type expression* grammar
(`type := union := …`, `array<T>`, unions, intersections, `mixed`, and so on) — this document does not
repeat it and adds no new production to it. What ADR 0007 left to the spec is the *statement*-level grammar
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
[ADR 0007 § 1](../adr/0007-explicit-type-system.md) already makes a diagnostic naming the first declaration.

### 3.2 `foreach` with typed bindings

```
foreach-stmt   := 'foreach' '(' expr 'as' foreach-target ')' block
foreach-target := type '$' identifier
                 | type '$' identifier '=>' type '&'? '$' identifier
```

```php
foreach ($names as string $name) { … }
foreach ($rows  as string $k => array<int> $row) { … }
foreach ($items as string $k => int &$v) { $v += 1; }        // reference binds the value only, as in PHP
```

Both the key and the value binding are typed — there is no untyped `foreach ($rows as $row)` left, the same
mandatory-declaration rule as everywhere else in ADR 0007. A reference marker (`&`) may only appear on the
value binding, never the key, matching PHP's own restriction.

**The grammar wrinkle ADR 0007 § 2 calls out by name:** inside a `foreach` header, `as` is `foreach`'s own
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

`list($a, $b) = $expr;` is accepted as a second spelling of `[$a, $b] = $expr` — kept for the same
pragmatic-superset reason `<?php` is kept as a second spelling of `<?mwl` — but every element inside it is
typed exactly as inside `[...]`; there is no untyped form of either spelling. Every leaf names its type
where it is bound, at any nesting depth, mirroring how `array<array<uint>>` nests in the type grammar itself.

### 3.4 The conversion operator: `as`, finally

[ADR 0007 § 2](../adr/0007-explicit-type-system.md) opens with "provisional spelling `expr as T`." This
document fixes that spelling as **final**: `as` is the conversion operator, with no alternate spelling, and
none is planned. Its precedence and throwing behaviour are exactly as ADR 0007 § 2 already states and are
not repeated here.

### 3.5 `type` aliases

```
type-alias-decl := 'type' ClassName '=' type ';'
```

Sits at file/namespace scope, alongside `use` and `namespace` — never inside a class body, per
[ADR 0015 § 5](../adr/0015-no-name-aliasing.md). `type` on its own is otherwise an ordinary reserved word in
this one declaration position; it is not a general statement keyword. The restriction that `TypeExpr` may
not be a single bare class/interface/enum atom ([ADR 0015 § 6](../adr/0015-no-name-aliasing.md)) is a
resolution-time check (M2), not a parse-time one — the grammar above parses `type Id = SomeClass;` exactly
like any other alias declaration, and M2's resolver is where it becomes a diagnostic.

## 4. The scoping surface (ADR 0008's spelling only)

[ADR 0008 § 2](../adr/0008-static-and-global.md) is the closed, exhaustive list of where state may outlive a
call, and this document adds no storage class to it and repeats none of its reasoning. What belongs here is
only the syntax for each row, gathered in one place since it is otherwise scattered across ADR 0007 and ADR
0008's own examples:

| storage | syntax |
|---|---|
| local variable, parameter | § 3.1 above; `function f(int $n) { … }` |
| class static property | `private static int $calls = 0;` |
| class constant | `public const int MAX = 10;` |
| object property | `public readonly uint $id;` |
| top-level script variable | § 3.1's grammar, written at file scope instead of inside a function |

`static` never appears as a declaration keyword outside a class member; there is no function-scope
`static int $x`, no `static fn`, and no `global` — each rejected with the diagnostic ADR 0008 § 5 already
names. This document fixes no new syntax for any of the three, because there is no replacement syntax to
fix: each rejection's replacement is one of the five rows above, already covered.

## 5. `bytes`: no dedicated literal

[ADR 0009](../adr/0009-string-and-bytes.md) leaves `bytes` literal syntax to this document. Decision: **there
is no `bytes` literal token.** The lexer needs no `b"…"`-shaped production, and M1's scope is smaller for it.

- For the common case — a byte sequence that happens to be valid UTF-8, which is most binary-ish constants a
  program writes by hand (magic strings, protocol markers made of ASCII) — a plain string literal converts
  for free: `"MWL1" as bytes`. [ADR 0009 § 3](../adr/0009-string-and-bytes.md) already makes `string as
  bytes` total and free, so this is not a new conversion rule, only its first literal-adjacent use.
- For a byte sequence that is **not** valid UTF-8 — a raw binary constant, a fixed hash or key material
  written inline — the spelling is a `Core\Bytes` constructor, following
  [ADR 0011](../adr/0011-functions-and-constants-are-class-members.md)'s "every callable is a class member"
  rule exactly as every other domain class does: `Core\Bytes::fromHex('deadbeef')`,
  `Core\Bytes::fromBase64('...')`. `Core\Bytes` joins the domain-class roster ADR 0011's summary names as
  examples, not as a closed list; building the class itself is ordinary M8 stdlib work, not part of this
  spec.

This resolves the open item in [ADR 0009 *Revisiting*](../adr/0009-string-and-bytes.md); that ADR's own
status (Proposed, pending the grapheme-cost guard test) is unaffected — this decision is about literal
syntax only, not about `string`'s default length/indexing granularity.

## Revisiting

- This document does not yet fix operator precedence beyond the two points ADR 0007 already pins (`as`
  binds tighter than any binary operator; `foreach`'s `as` versus the conversion operator). A full
  precedence table belongs in a later spec file once the parser needs one written down rather than inferred
  from the grammar productions above.
- `spawn`/`await`/`Channel`/`spawn worker` and the rest of the concurrency surface are named here only where
  `spawn script`'s grammar needs to distinguish itself from them; M5 is where they get their own spec
  section.
- If `.phpt`-corpus parsing (M1's verification) turns up a real PHP file using a short open tag (bare `<?`),
  that is a decision to make then, not a gap in this document — MWL never had one to begin with, so adding
  one would be a new grammar choice, not a restatement of an existing one.
