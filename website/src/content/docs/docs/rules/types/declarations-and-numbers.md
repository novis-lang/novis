---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Declarations and numbers"
description: "Every binding declares its type and keeps it. The numeric tower, its one implicit conversion, and what overflow does."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/types/
  label: "Types"
next:
  link: /docs/rules/types/text-and-literal-types/
  label: "Text, bytes and literal types"
---

<p class="nv-section-lead">Every binding declares its type and keeps it. The numeric tower, its one implicit conversion, and what overflow does.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">9</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">9</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">7</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#declaration">Every binding declares its type, and no binding's type ever changes</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#var-inference"><code>var</code> takes a local's type from its initializer and fixes it there for good</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#grammar">The type grammar is a closed set of atoms under unions and intersections</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#integer-literals">An integer literal is decimal, <code>0x</code>, <code>0o</code> or <code>0b</code>, and a leading zero is not a radix</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#numeric-literal-placement">A numeric literal is untyped until it is placed, and <code>as T</code> is a placing position</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#uint"><code>uint</code> is a distinct unsigned 64-bit integer, and it costs no bytes a value did not already hold</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#arithmetic">An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#implicit-widening">An <code>int</code> or <code>uint</code> widening into a <code>float</code> position is the only implicit conversion in the language</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#decimal"><code>decimal</code> is an exact scalar of 96 mantissa bits and a scale of 0 to 28</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="declaration">

## Every binding declares its type, and no binding's type ever changes

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#declaration"><code>types/declaration</code></a>
</div>

Every binding site carries a written type. PHP's existing slots become mandatory — parameter, return
(`void` and `never` included), property, promoted constructor parameter, class constant, closure
parameter and closure return, `catch`, enum backing type — and four positions PHP has no slot for get
one: a local at its declaration, a `foreach` key and value, a `for` header's init clause
([`iteration/for-init-clause`](/docs/rules/iteration/#for-init-clause "A for init clause is one typed declaration or a list of expressions, never both")), and a destructuring target. A local may write `var` instead
([`types/var-inference`](/docs/rules/types/declarations-and-numbers/#var-inference "var takes a local's type from its initializer and fixes it there for good")); nothing else may omit a type.

The return slot is owed by every declaration a caller reads, an abstract method and an interface
member included, and **the constructor is the one exception**: it answers with the instance rather
than with a value, which is why a valued `return` in one is refused, so it writes no return type and
a written `: void` there is accepted while saying nothing the declaration did not. An
expression-bodied closure is the other place the slot may stand empty, and for the opposite reason —
its body is a single expression, which is its own answer, while a block-bodied one owes the
annotation like any method ([`types/closure-literal`](/docs/rules/types/closures/#closure-literal "fn is the only closure literal, with an expression body or a block body")).

A binding is declared **once**. A later assignment is bare, and is legal only where the name is
already declared in the enclosing function; re-declaring a live name is a diagnostic naming the first
declaration, and there is no shadowing. Declaration is function-scoped as in PHP — a binding declared
inside an `if` is visible after it — but *definite assignment is checked*: reading a binding on a path
that may not have reached its initialiser is a compile error, not PHP's warning and a `null`.

A declared type is then fixed for the binding's whole life. No assignment, operator or call changes
it; `settype()` joins the rejected list with a diagnostic naming `as`
([`types/conversion`](/docs/rules/types/unions-and-conversion/#conversion "expr as T is the only conversion, and it produces a T or throws")). An `inout` binding ties two names to one slot, so both sides declare the
**same** type ([`statements/inout-is-the-by-reference-spelling`](/docs/rules/statements/by-reference-and-exit/#inout-is-the-by-reference-spelling "A by-reference binding is written inout, in the modifier slot before the type")); an alias that widens or narrows
is a diagnostic. There is no function-scope `static` and no global constant, so neither has a binding
site at all ([`statements/no-function-static-and-no-global`](/docs/rules/statements/where-state-lives/#no-function-static-and-no-global "A function holds no state of its own, and global does not exist")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A local, a <code>foreach</code> binding, a <code>for</code> counter and a destructuring target all carry a written type, and PHP source with none does not compile</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#var-inference" title="var takes a local's type from its initializer and fixes it there for good"><code>types/var-inference</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/declarations-and-numbers/#grammar" title="The type grammar is a closed set of atoms under unions and intersections"><code>types/grammar</code></a> <a href="/docs/rules/types/unions-and-conversion/#unions-and-mixed" title="A union permits only what every member permits, and mixed is the one position checked nowhere"><code>types/unions-and-mixed</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0022.md">record 0022</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0037.md">record 0037</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0008.md">record 0008</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0011.md">record 0011</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-locals-type-is-fixed-at-its-declaration.nvst"><code>tests/conformance/lang/a-locals-type-is-fixed-at-its-declaration.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-declaration-without-an-initializer-fixes-the-type.nvst"><code>tests/conformance/lang/a-declaration-without-an-initializer-fixes-the-type.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-compound-assignment-cannot-retype-its-target.nvst"><code>tests/conformance/reject/a-compound-assignment-cannot-retype-its-target.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/locals.rs"><code>crates/nvs-types/tests/locals.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="var-inference">

## `var` takes a local's type from its initializer and fixes it there for good

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#var-inference"><code>types/var-inference</code></a>
</div>

`var $name = expr;` declares a local without writing its type. The type stored on the binding is
`expr`'s own checked type, computed exactly as it is for any position with no expected type, and it is
then fixed on the binding like a written one — [`types/declaration`](/docs/rules/types/declarations-and-numbers/#declaration "Every binding declares its type, and no binding's type ever changes")'s "no binding's type ever
changes" is untouched.

The initializer is mandatory: `var $n;` is a parse error naming `=`. The inferred type is exact and
honest — `var $n = 1;` gives `int`, and `var $id = Core\Request::query('id');` gives `mixed`, because
that is what the initializer is.

One initializer shape is refused: a **bare array literal**. `var $x = [1, 2];` is
`E_VAR_ARRAY_LITERAL_NEEDS_TYPE`, naming `array<T> $x = [1, 2];` as the fix, because an array literal
is checked against a target rather than inferring one ([`types/arrays`](/docs/rules/types/arrays-and-property-keys/#arrays "An array is PHP's ordered hash with string keys and a declared element type")). The restriction is on
the initializer's own top level only — `var $x = f([1, 2]);` is fine, since `f`'s parameter is the
literal's target.

Every other rule that applies to a typed local declaration — declare-once, definite assignment,
redeclaration diagnostics — applies unchanged, because by the time those checks run `var` has already
resolved to a concrete type.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#declaration" title="Every binding declares its type, and no binding's type ever changes"><code>types/declaration</code></a> <a href="/docs/rules/types/declarations-and-numbers/#numeric-literal-placement" title="A numeric literal is untyped until it is placed, and as T is a placing position"><code>types/numeric-literal-placement</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#arrays" title="An array is PHP's ordered hash with string keys and a declared element type"><code>types/arrays</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0037.md">record 0037</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0114.md">record 0114</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/var-infers-from-its-initializer.nvst"><code>tests/conformance/lang/var-infers-from-its-initializer.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/var-refuses-a-bare-array-literal.nvst"><code>tests/conformance/lang/var-refuses-a-bare-array-literal.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/locals.rs"><code>crates/nvs-types/tests/locals.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="grammar">

## The type grammar is a closed set of atoms under unions and intersections

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#grammar"><code>types/grammar</code></a>
</div>

```
type         := qualified
qualified    := ('secret')? ('tainted')? union
union        := intersection ('|' intersection)*
intersection := atom ('&' atom)*  |  '(' union ')'        // DNF, as PHP 8.2
atom         := 'null' | 'bool' | 'int' | 'uint' | 'float' | 'decimal'
              | 'string' | 'bytes'
              | 'array' | 'array' '<' type '>'
              | 'class' '<' Name '>'
              | 'property' '<' Name '>'
              | 'object' | 'mixed' | 'void' | 'never' | 'true' | 'false'
              | 'iterable' | 'callable' | 'self' | 'static' | 'parent'
              | 'callable' '(' (type (',' type)*)? ')' ':' type
              | StringLiteral | IntLiteral
              | '{' field (',' field)* '}'
              | Name
              | '?' atom                                  // sugar for atom|null
field        := identifier ':' type
```

Unions are canonicalised — flattened, de-duplicated, order-insensitive — so `int|string` and
`string|int|int` are one type. `array` with no argument is exactly `array<mixed>`. `void` and `never`
are return-only. `array<T>` is parsed **only in type position**, where a `<` is unambiguously a
type-argument list; two expression positions also admit one, a call site's own `<...>` and a `new`
target's, both settled by a checkpointed trial parse that commits only when the list parses cleanly
and a `(` follows, so `new Foo < $x` stays a comparison. Only a compiler-owned generic declaration may
carry one.

`Name` covers four kinds of atom told apart by resolution: a class or interface name, an enum's name,
an enum case ([`types/enum-case-type`](/docs/rules/types/text-and-literal-types/#enum-case-type "An enum case used as a type is a narrowed subtype of its enum, never its backing integer")), and a `type` alias ([`types/type-alias`](/docs/rules/types/unions-and-conversion/#type-alias "A type alias is a transparent, compile-time-only synonym for a type expression")). A class name
carries concrete arguments only where it names a compiler-owned generic interface — `Iterator<User>`
([`iteration/concrete-generic-implements`](/docs/rules/iteration/#concrete-generic-implements "A class implements a compiler-owned generic interface at a concrete type, and declares no type variable of its own")) — and nowhere else.

**There is no `resource` type.** A host handle is an ordinary object with an explicit `close()`.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#declaration" title="Every binding declares its type, and no binding's type ever changes"><code>types/declaration</code></a> <a href="/docs/rules/types/unions-and-conversion/#unions-and-mixed" title="A union permits only what every member permits, and mixed is the one position checked nowhere"><code>types/unions-and-mixed</code></a> <a href="/docs/rules/types/unions-and-conversion/#type-alias" title="A type alias is a transparent, compile-time-only synonym for a type expression"><code>types/type-alias</code></a> <a href="/docs/rules/types/objects-and-shapes/#shape-type" title="{name: T} in type position is a structural shape checked by width subtyping, and {name?: T} marks a key that may be absent"><code>types/shape-type</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0009.md">record 0009</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0015.md">record 0015</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0031.md">record 0031</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0036.md">record 0036</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0047.md">record 0047</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0053.md">record 0053</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0054.md">record 0054</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0125.md">record 0125</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0126.md">record 0126</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0136.md">record 0136</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-syntax/src/parser/tests/ty.rs"><code>crates/nvs-syntax/src/parser/tests/ty.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-ir/tests/type_atoms.rs"><code>crates/nvs-ir/tests/type_atoms.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/object-is-a-declared-type-in-every-position.nvst"><code>tests/conformance/lang/object-is-a-declared-type-in-every-position.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-core-collection-takes-the-type-arguments-it-declares.nvst"><code>tests/conformance/lang/a-core-collection-takes-the-type-arguments-it-declares.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="integer-literals">

## An integer literal is decimal, `0x`, `0o` or `0b`, and a leading zero is not a radix

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#integer-literals"><code>types/integer-literals</code></a>
</div>

An integer literal is written decimal, `0x`, `0o` or `0b` — either case of the prefix letter, with `_`
separators allowed between digits. Those four are the closed set, so **a leading zero is not a
radix**: `017` is decimal seventeen where PHP reads octal fifteen, and `0o17` is the only spelling of
that fifteen. PHP's legacy form is refused as a *silent* reinterpretation rather than as a spelling —
it changes a value without changing a character, which is the one thing a converted file cannot be
checked for, and a file-mode constant is where it bites.

**There is no literal suffix, for any numeric type.** A literal takes `int`, `uint`, `float` or
`decimal` from the position it is written in instead ([`types/numeric-literal-placement`](/docs/rules/types/declarations-and-numbers/#numeric-literal-placement "A numeric literal is untyped until it is placed, and as T is a placing position")), and a
literal too wide for `int` is legal only where a `uint` is expected.

The escape grammar inside a string literal is unrelated to this and matches PHP's exactly, `\v`, `\f`,
`\e` and the octal `\0`–`\777` included.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>017</code> is decimal seventeen where PHP reads octal fifteen; PHP's leading-zero octal is refused rather than reinterpreted</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#uint" title="uint is a distinct unsigned 64-bit integer, and it costs no bytes a value did not already hold"><code>types/uint</code></a> <a href="/docs/rules/types/declarations-and-numbers/#numeric-literal-placement" title="A numeric literal is untyped until it is placed, and as T is a placing position"><code>types/numeric-literal-placement</code></a> <a href="/docs/rules/types/declarations-and-numbers/#arithmetic" title="An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting"><code>types/arithmetic</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-integer-literal-takes-uint-from-the-operand-beside-it.nvst"><code>tests/conformance/lang/an-integer-literal-takes-uint-from-the-operand-beside-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-syntax/src/lexer.rs"><code>crates/nvs-syntax/src/lexer.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="numeric-literal-placement">

## A numeric literal is untyped until it is placed, and `as T` is a placing position

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#numeric-literal-placement"><code>types/numeric-literal-placement</code></a>
</div>

A numeric literal is **untyped until it is placed**, and takes its type from the position it appears
in. An integer literal becomes `int`, `uint`, `float` or `decimal`; a literal carrying a fractional
part or an exponent becomes `decimal` or `float`. Because every binding site declares a type
([`types/declaration`](/docs/rules/types/declarations-and-numbers/#declaration "Every binding declares its type, and no binding's type ever changes")), the target is known almost everywhere.

```php
decimal $price = 19.99;          // exact: mantissa 1999, scale 2
float   $ratio = 19.99;          // an f64
var $x = 19.99;                  // no target type: float
var $y = 19.99 as decimal;       // `as` supplies one: decimal, exact
```

**`expr as T` is itself a placing position.** A literal written directly under a conversion takes `T`
as its target rather than being typed first and converted afterwards, so `19.99 as decimal` is exact
to the full 29 significant digits and never becomes an `f64` on the way. That is not merely notational:
`float → decimal` recovers only the ~17 digits an `f64` round-trips, so without this rule a wider
literal would be unwritable in any position lacking an annotation.

**There is no literal suffix, and in particular no `m`** ([`types/integer-literals`](/docs/rules/types/declarations-and-numbers/#integer-literals "An integer literal is decimal, 0x, 0o or 0b, and a leading zero is not a radix")). A suffix
would buy only a second spelling of what `as decimal` already says, in the two positions that lack a
target: a `var` declaration ([`types/var-inference`](/docs/rules/types/declarations-and-numbers/#var-inference "var takes a local's type from its initializer and fixes it there for good")) and a `mixed` or generic argument.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A fractional literal is not born a <code>float</code> — <code>decimal $price = 19.99;</code> is exact — and there is no literal suffix for any numeric type</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#decimal" title="decimal is an exact scalar of 96 mantissa bits and a scale of 0 to 28"><code>types/decimal</code></a> <a href="/docs/rules/types/declarations-and-numbers/#integer-literals" title="An integer literal is decimal, 0x, 0o or 0b, and a leading zero is not a radix"><code>types/integer-literals</code></a> <a href="/docs/rules/types/declarations-and-numbers/#var-inference" title="var takes a local's type from its initializer and fixes it there for good"><code>types/var-inference</code></a> <a href="/docs/rules/types/declarations-and-numbers/#implicit-widening" title="An int or uint widening into a float position is the only implicit conversion in the language"><code>types/implicit-widening</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0054.md">record 0054</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0037.md">record 0037</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-integer-literal-takes-uint-from-the-operand-beside-it.nvst"><code>tests/conformance/lang/an-integer-literal-takes-uint-from-the-operand-beside-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-literal-adapts-to-a-generic-uint.nvst"><code>tests/conformance/core/a-literal-adapts-to-a-generic-uint.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/decimal.rs"><code>crates/nvs-types/tests/decimal.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="uint">

## `uint` is a distinct unsigned 64-bit integer, and it costs no bytes a value did not already hold

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#uint"><code>types/uint</code></a>
</div>

`uint` is an unsigned 64-bit integer, `0 … 2^64−1`. It is a new **tag** in the existing tagged value
whose payload is already a `u64`, so a `uint` costs **zero additional bytes per value**.
`Core\Reflect::typeOf` reports it as its own kind, and `$x is uint` asks for it directly
([`types/type-test`](/docs/rules/types/unions-and-conversion/#type-test "$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable")).

Being its own tag is observable, and a migrating program is where it shows: `is_int($id)` was true for
every integer PHP had, while `$id is int` is **false** for a value that arrived as a `uint` — a
`BIGINT UNSIGNED` key or a snowflake id, which is what `uint` was added for. `$id is int|uint` is the
spelling that asks PHP's question. This is the one place the split is reachable by a mechanical
rewrite rather than by declaring a `uint` on purpose.

`uint` exists because web software needs the half of the 64-bit range PHP's single signed integer
cannot reach: `BIGINT UNSIGNED` keys, snowflake ids, nanosecond timestamps, WIT's `u32`/`u64`. An
integer literal that does not fit `int` is legal only where a `uint` is expected, and is otherwise a
diagnostic saying exactly that.

`int` and `uint` are separate types everywhere it matters. They mix in a comparison, which has an
exact answer over the mathematical integers, and they do not mix in arithmetic, which has no
representable common type to return ([`types/arithmetic`](/docs/rules/types/declarations-and-numbers/#arithmetic "An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting")). Converting between them is `as`, and it
throws rather than wrapping ([`types/conversion`](/docs/rules/types/unions-and-conversion/#conversion "expr as T is the only conversion, and it produces a T or throws")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP has one integer type; Novis has two, told apart by <code>Core\Reflect::typeOf</code> and by <code>$x is uint</code> (<code>rule:types/type-test</code>), so a value PHP's <code>is_int()</code> accepted answers <code>is uint</code> and not <code>is int</code>, and the whole 64-bit range is representable</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#arithmetic" title="An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting"><code>types/arithmetic</code></a> <a href="/docs/rules/types/declarations-and-numbers/#integer-literals" title="An integer literal is decimal, 0x, 0o or 0b, and a leading zero is not a radix"><code>types/integer-literals</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/unions-and-conversion/#type-test" title="$x is T tests whether a value holds a T, answers bool, and never refuses because the answer is knowable"><code>types/type-test</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0004.md">record 0004</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0150.md">record 0150</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/uint-spans-its-whole-range.nvst"><code>tests/conformance/lang/uint-spans-its-whole-range.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/uint-arithmetic-reaches-the-top-of-its-range.nvst"><code>tests/conformance/lang/uint-arithmetic-reaches-the-top-of-its-range.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="arithmetic">

## An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#arithmetic"><code>types/arithmetic</code></a>
</div>

| operation | result | on overflow / edge |
|---|---|---|
| `int ⊕ int`, `uint ⊕ uint` for `+ - * ** %` | the same type | **throws `ArithmeticError`.** No wrap, no promotion to `float` |
| `int ⊕ uint` arithmetic | **compile error** | there is no representable common type; convert one side explicitly |
| `int / int`, `uint / uint` | `int\|float`, `uint\|float` — PHP-exact: `6/3` is an integer, `7/2` is a float | `/ 0` throws `ArithmeticError` |
| either operand a `float` | `float` for `+ - * ** /`; **`%` is a compile error** | `/ 0` throws here too — the zero divisor is refused before the operand types are consulted. IEEE division is `Core\Math::fdiv` |
| `decimal ⊕ decimal`, `decimal ⊕ int`, `decimal ⊕ uint` for `+ - * %` | `decimal` | throws when the mantissa exceeds 96 bits **or** the scale would exceed 28 |
| `decimal / decimal` | `decimal`, half-even at the maximum scale the result admits | `/ 0` throws |
| `decimal ⊕ float`; `**` with a `decimal` base | **compile error** | no representable common type; `Core\Decimal::pow` for the power |
| `>>` | arithmetic on `int`, **logical on `uint`** | — |
| `& \| ^ ~ <<` | the operand type, preserved | — |

The arithmetic rows are a **closed** list. Their operands are `int`, `uint`, `float` and `decimal`, so
a `bool`, a `string`, a `bytes`, an `array<T>`, a `callable`, `null` and an object have no `+` at all
and are refused where they are written. `%` is narrower than its own float row: a `float` operand is
refused rather than given one of two plausible answers, and `Core\Math::mod` is the member that says
the floating-point remainder out loud.

Division is the one row that returns a union, and in practice the target's declared type absorbs it
through the `int → float` widening ([`types/implicit-widening`](/docs/rules/types/declarations-and-numbers/#implicit-widening "An int or uint widening into a float position is the only implicit conversion in the language")): `float $avg = $sum / $n;` works,
`int $n = 7 / 2;` is a diagnostic, and `Core\Math::intDiv` is there when integer division was meant.

An operand whose static type names no row — `mixed`, a union, the `int|float` a division returns — is
answered from its runtime **tag**: the rows above where the tags name one, and the same refusal as a
*catchable throw* where they do not, carrying the diagnostic's own wording.

Overflow throwing is the divergence this table is least willing to trade. A silent promotion to
`float` changes a binding's type behind its declaration, and a silent wrap is the classic
size-computation bug. Code that wants unbounded magnitude declares `float`, or converts.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>PHP_INT_MAX + 1</code> throws <code>ArithmeticError</code> instead of becoming a <code>float</code>, <code>int</code> against <code>uint</code> does not compile, and a <code>bool</code>, <code>string</code>, <code>array&lt;T&gt;</code> or object operand has no <code>+</code> at all</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/text-and-literal-types/#ordering" title="Only the types the table orders may be ordered, and two objects need Comparable"><code>types/ordering</code></a> <a href="/docs/rules/types/declarations-and-numbers/#uint" title="uint is a distinct unsigned 64-bit integer, and it costs no bytes a value did not already hold"><code>types/uint</code></a> <a href="/docs/rules/types/declarations-and-numbers/#decimal" title="decimal is an exact scalar of 96 mantissa bits and a scale of 0 to 28"><code>types/decimal</code></a> <a href="/docs/rules/types/declarations-and-numbers/#implicit-widening" title="An int or uint widening into a float position is the only implicit conversion in the language"><code>types/implicit-widening</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0054.md">record 0054</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0010.md">record 0010</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0035.md">record 0035</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/the-arithmetic-table-is-closed.nvst"><code>tests/conformance/lang/the-arithmetic-table-is-closed.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-integer-overflow-throws-rather-than-wrapping.nvst"><code>tests/conformance/lang/an-integer-overflow-throws-rather-than-wrapping.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/mixed-signedness-arithmetic-is-a-compile-error.nvst"><code>tests/conformance/lang/mixed-signedness-arithmetic-is-a-compile-error.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/every-arithmetic-row-promotes-the-narrower-operand.nvst"><code>tests/conformance/lang/every-arithmetic-row-promotes-the-narrower-operand.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/arithmetic-over-a-mixed-operand-is-decided-by-its-tag.nvst"><code>tests/conformance/lang/arithmetic-over-a-mixed-operand-is-decided-by-its-tag.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/an-object-takes-part-in-no-arithmetic.nvst"><code>tests/conformance/reject/an-object-takes-part-in-no-arithmetic.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="implicit-widening">

## An `int` or `uint` widening into a `float` position is the only implicit conversion in the language

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#implicit-widening"><code>types/implicit-widening</code></a>
</div>

Implicit conversion happens in exactly one place: an `int` or `uint` **widening into a `float`
position** — an argument, a return, an assignment, or the far side of an arithmetic operator. It is
the one coercion PHP's own `strict_types` permits, and it throws above 2^53 rather than rounding,
where `f64` stops representing every integer.

Everything else is a diagnostic. `mixed` never absorbs implicitly in either direction
([`types/unions-and-mixed`](/docs/rules/types/unions-and-conversion/#unions-and-mixed "A union permits only what every member permits, and mixed is the one position checked nowhere")), a `decimal` never meets a `float` in arithmetic
([`types/decimal`](/docs/rules/types/declarations-and-numbers/#decimal "decimal is an exact scalar of 96 mantissa bits and a scale of 0 to 28")), and every remaining change of type is written as `as`
([`types/conversion`](/docs/rules/types/unions-and-conversion/#conversion "expr as T is the only conversion, and it produces a T or throws")). A numeric literal is not a conversion at all: it is untyped until placed,
so it takes `int`, `uint`, `float` or `decimal` from its target ([`types/numeric-literal-placement`](/docs/rules/types/declarations-and-numbers/#numeric-literal-placement "A numeric literal is untyped until it is placed, and as T is a placing position")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The widening throws above 2^53 instead of rounding, and no other implicit coercion survives</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/declarations-and-numbers/#arithmetic" title="An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting"><code>types/arithmetic</code></a> <a href="/docs/rules/types/declarations-and-numbers/#numeric-literal-placement" title="A numeric literal is untyped until it is placed, and as T is a placing position"><code>types/numeric-literal-placement</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0054.md">record 0054</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/every-arithmetic-row-promotes-the-narrower-operand.nvst"><code>tests/conformance/lang/every-arithmetic-row-promotes-the-narrower-operand.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/arr-a-callback-float-parameter-widens-an-int-and-stops-at-2-53.nvst"><code>tests/conformance/core/arr-a-callback-float-parameter-widens-an-int-and-stops-at-2-53.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="decimal">

## `decimal` is an exact scalar of 96 mantissa bits and a scale of 0 to 28

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#decimal"><code>types/decimal</code></a>
</div>

`decimal` is a scalar: a sign, a 96-bit unsigned mantissa, and a scale of 0 to 28 giving the digits
after the point. Its value is `(-1)^sign × mantissa × 10^-scale` — roughly 29 significant digits. It
is register-pair sized, allocation-free and refcount-free, and it costs **16 bytes per value** against
8 for a `float`.

It is deliberately **not** arbitrary precision. World GDP in cents is 17 digits; Bitcoin to satoshis
is 16. What lies beyond is `Core\BigDecimal` — arbitrary-precision, heap-allocated, method-based, no
literal form — named here so the boundary is stated rather than discovered.

Scale is carried for rendering and does not affect equality or hashing: `1.10 == 1.1000` is true, and
`19.90` renders `"19.90"`. **Division is the one operation that may be inexact**, and its policy is
fixed in the language and not configurable: round half to even, at the maximum scale the result
admits. There is no `bcscale()` equivalent and never will be. Where rounding is business logic it is
said out loud — `Core\Decimal::divExact()` throws unless the quotient is exact,
`::divRound($scale, $mode)` names both, and `::allocate($amount, $ratios)` splits a sum into parts
that add back to it exactly.

`bcmath` and `gmp` are retired rather than ported, because they conflated two unrelated capabilities:
exact fractional arithmetic at human magnitudes, which is `decimal`, and arbitrary-magnitude integers
wearing a decimal API, which is `Core\BigInt`.

`decimal` is not an enum backing type ([`enums/one-backing-type`](/docs/rules/enums/#one-backing-type "Every enum is backed by exactly one integer type, and nothing else backs one")), and array keys are unaffected —
every key is a `string` already ([`types/arrays`](/docs/rules/types/arrays-and-property-keys/#arrays "An array is PHP's ordered hash with string keys and a declared element type")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>bcmath</code> and <code>gmp</code> are retired rather than ported — exact fractional arithmetic is a register-sized scalar, and arbitrary-magnitude integers are <code>Core\BigInt</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#numeric-literal-placement" title="A numeric literal is untyped until it is placed, and as T is a placing position"><code>types/numeric-literal-placement</code></a> <a href="/docs/rules/types/declarations-and-numbers/#arithmetic" title="An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting"><code>types/arithmetic</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0054.md">record 0054</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0004.md">record 0004</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/decimal-arithmetic-is-exact-and-keeps-its-scale.nvst"><code>tests/conformance/lang/decimal-arithmetic-is-exact-and-keeps-its-scale.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/decimal-conversions-are-checked-in-both-directions.nvst"><code>tests/conformance/lang/decimal-conversions-are-checked-in-both-directions.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/decimal-division-refuses-a-zero-divisor-and-a-scale-it-cannot-hold.nvst"><code>tests/conformance/core/decimal-division-refuses-a-zero-divisor-and-a-scale-it-cannot-hold.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/decimal.rs"><code>crates/nvs-types/tests/decimal.rs</code></a></dd></div></dl>

</div>
