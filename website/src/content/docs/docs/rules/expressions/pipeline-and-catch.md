---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "The pipeline and the catch expression"
description: "Two constructs PHP has no spelling for: a parse-time substitution, and a typed catch that guards one expression."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/expressions/conversion-and-intrinsics/
  label: "Conversion and the intrinsics"
next:
  link: /docs/rules/statements/
  label: "Statements"
---

<p class="nv-section-lead">Two constructs PHP has no spelling for: a parse-time substitution, and a typed catch that guards one expression.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">10</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">7</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">3</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">3</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#pipeline-substitution"><code>|&gt;</code> substitutes one hole at parse time, and has no run-time representation</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#pipeline-precedence"><code>|&gt;</code> binds tighter than every binary operator and looser than unary</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#pipeline-hole-once">The hole is <code>$_</code>, it appears exactly once on a right side, and nowhere else at all</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#catch-expression">An expression-level <code>catch</code> is a chain of typed arms guarding one expression</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#catch-expression-precedence"><code>catch</code> sits between assignment and the ternary, so one arm guards a whole <code>??</code> chain</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#catch-arm-is-an-expression">An arm body is an expression, which admits <code>throw</code> and refuses <code>return</code> by name</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#catch-result-type">The result type is the union of the guard and every arm, and an arm is checked from the pre-guard state</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#bare-throwable-arm-warns">An unbound <code>catch (Throwable)</code> arm that supplies a value is warned about</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#catch-lowers-to-block-form">The expression form lowers to the block form's landing pad, with the value in a temporary</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#first-class-callable-syntax"><code>Name(...)</code> is the only spelling that takes a reference to a declared member</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="pipeline-substitution">

## `|>` substitutes one hole at parse time, and has no run-time representation

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#pipeline-substitution"><code>expressions/pipeline-substitution</code></a>
</div>

`|>` is a binary operator whose right side is an expression containing the hole `$_`. The **parser**
replaces that hole with the left side and emits the resulting expression. There is no run-time
representation of `|>`, and no callable is involved.

```
PipeExpr := Unary ( "|>" PipeRhs )*
PipeRhs  := Postfix                     // a call, an index, a member access, or a parenthesized group
```

`$subject |> RHS` produces `RHS` with its `$_` replaced by `$subject`, left-associative, so
`$a |> f($_) |> g($_)` is `g(f($a))`. The right side is parsed at the postfix level, so an expression
that is not already a call or an access is written parenthesized: `$n |> ($_ * 2)`.

Because the result is the tree the nested spelling produces, every later pass — name resolution, the
type checker, taint and `secret`, literal folding, lowering, codegen — sees a node it already handles.
Nothing about the `Core` roster, scalar methods or any security property changes: `$userTemplate |>
Str::format($_, $n)` is refused for the same reason the nested call is. There is no closure allocated
and no dynamic dispatch.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP 8.5's <code>|&gt;</code> applies a callable resolved at run time; this one substitutes <code>$_</code> in the parser, so the emitted tree is the nested spelling's and no closure is allocated</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-hole-once" title="The hole is $_, it appears exactly once on a right side, and nowhere else at all"><code>expressions/pipeline-hole-once</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-precedence" title="|&gt; binds tighter than every binary operator and looser than unary"><code>expressions/pipeline-precedence</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#first-class-callable-syntax" title="Name(...) is the only spelling that takes a reference to a declared member"><code>expressions/first-class-callable-syntax</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0098.md">record 0098</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0011.md">record 0011</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0027.md">record 0027</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div></dl>

</div>

<div class="nv-rule" id="pipeline-precedence">

## `|>` binds tighter than every binary operator and looser than unary

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#pipeline-precedence"><code>expressions/pipeline-precedence</code></a>
</div>

`|>` binds **tighter than every binary operator and looser than unary**. The four cases that fixes,
each of which a looser placement gets wrong:

| written | groups as |
|---|---|
| `-$a \|> Math::abs($_)` | `Math::abs(-$a)` |
| `"x=" . $a \|> Str::upper($_)` | `"x=" . Str::upper($a)` |
| `$a \|> Str::length($_) > 5` | `Str::length($a) > 5` |
| `$x = $a \|> Str::trim($_)` | `$x = Str::trim($a)` |

Substituting a single hole commutes with the surrounding operator, so `$a |> Str::upper($_) . "!"`
groups as `Str::upper($a) . "!"` and the parenthesis trap a callable-applying pipeline has cannot
arise.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-substitution" title="|&gt; substitutes one hole at parse time, and has no run-time representation"><code>expressions/pipeline-substitution</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-hole-once" title="The hole is $_, it appears exactly once on a right side, and nowhere else at all"><code>expressions/pipeline-hole-once</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#catch-expression-precedence" title="catch sits between assignment and the ternary, so one arm guards a whole ?? chain"><code>expressions/catch-expression-precedence</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0098.md">record 0098</a></dd></div></dl>

</div>

<div class="nv-rule" id="pipeline-hole-once">

## The hole is `$_`, it appears exactly once on a right side, and nowhere else at all

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#pipeline-hole-once"><code>expressions/pipeline-hole-once</code></a>
</div>

The hole is `$_`. It wears a `$` because it is a binding, and in this language a binding wears a `$`.
The parser turns `$_` into its own node before identifier casing sees a variable, so the rule that
rejects an all-underscore identifier does not fire on it.

**`$_` appears exactly once on a right side** — not at least once. Three refusals keep that
enforceable, and `crates/nvs-diagnostics/src/lib.rs` is the registry that allocates their numbers:

| code | when | what it says |
|---|---|---|
| `E0129` | a right side of `\|>` contains no `$_` | names the shape (`Str::trim($_)`) and, when the right side is first-class callable syntax or a closure value, adds that this `\|>` substitutes a hole rather than applying a callable |
| `E0130` | `$_` appears more than once on one right side | names binding the value to a local instead |
| `E0131` | `$_` appears anywhere outside the right side of a `\|>` | says the hole has no meaning there |

Exactly one hole is what makes substitution total: no temporary, no double evaluation, and an emitted
tree identical to the nested spelling's.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-substitution" title="|&gt; substitutes one hole at parse time, and has no run-time representation"><code>expressions/pipeline-substitution</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-precedence" title="|&gt; binds tighter than every binary operator and looser than unary"><code>expressions/pipeline-precedence</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0098.md">record 0098</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0030.md">record 0030</a></dd></div></dl>

</div>

<div class="nv-rule" id="catch-expression">

## An expression-level `catch` is a chain of typed arms guarding one expression

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#catch-expression"><code>expressions/catch-expression</code></a>
</div>

A `catch` arm is a postfix on an expression. It names one class, optionally binds the thrown object,
and supplies an expression whose value stands in where the guarded expression threw.

```
CatchExpr := Ternary ( "catch" "(" Type Variable? ")" "=>" Ternary )*
```

```
var $cfg = Core\Json::parse($raw) catch (ParseError) => [];
```

The class is mandatory and is one class or interface under `Throwable`, resolved and checked exactly
as a block clause's is — the same codes for a name that is not a class, for a class that is not a
`Throwable`, and for a limit report, which is not catchable here either
([`errors/throwable-hierarchy`](/docs/rules/errors/how-an-error-travels/#throwable-hierarchy "A limit report is not a Throwable, and the type checker knows it")). A union is refused with the block form's own wording: *write two
arms*. The binding is optional and, where written, is a local of the enclosing function under the
block form's rule.

**Arms are clauses of one guard, not guards of each other.** `f() catch (A) => x catch (B) => y` tries
`A` then `B` against what `f()` threw, and `x` is not guarded by the `B` arm. A supertype written
first shadows the arms after it, as in the block form.

The block form is unchanged in every particular and remains the only spelling with `finally`. Resource
cleanup stays there; this form has no one-line twin for it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP has no expression-level <code>catch</code>; a one-line fallback for a throwing call is a language form rather than a swallowed <code>@</code> or a five-line block</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/pipeline-and-catch/#catch-arm-is-an-expression" title="An arm body is an expression, which admits throw and refuses return by name"><code>expressions/catch-arm-is-an-expression</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#catch-result-type" title="The result type is the union of the guard and every arm, and an arm is checked from the pre-guard state"><code>expressions/catch-result-type</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#catch-expression-precedence" title="catch sits between assignment and the ternary, so one arm guards a whole ?? chain"><code>expressions/catch-expression-precedence</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#catch-lowers-to-block-form" title="The expression form lowers to the block form's landing pad, with the value in a temporary"><code>expressions/catch-lowers-to-block-form</code></a> <a href="/docs/rules/errors/how-an-error-travels/#throwable-hierarchy" title="A limit report is not a Throwable, and the type checker knows it"><code>errors/throwable-hierarchy</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0119.md">record 0119</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0002.md">record 0002</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-expression-catch-yields-the-arm-where-the-guard-threw.nvst"><code>tests/conformance/lang/an-expression-catch-yields-the-arm-where-the-guard-threw.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/catch-arms-are-tried-in-order-and-the-rest-is-rethrown.nvst"><code>tests/conformance/lang/catch-arms-are-tried-in-order-and-the-rest-is-rethrown.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-catch-arm-names-one-class-and-a-union-asks-for-two-arms.nvst"><code>tests/conformance/reject/a-catch-arm-names-one-class-and-a-union-asks-for-two-arms.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/catch_expression.rs"><code>crates/nvs-types/tests/catch_expression.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="catch-expression-precedence">

## `catch` sits between assignment and the ternary, so one arm guards a whole `??` chain

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#catch-expression-precedence"><code>expressions/catch-expression-precedence</code></a>
</div>

`catch` sits **between assignment and the ternary**. The guarded expression is everything the ternary
level parses, so one arm covers a whole `??` chain or a whole `?:`; the arm body is parsed at the same
level, so a following `catch` starts the next arm of the same guard rather than nesting under the
fallback.

| written | groups as |
|---|---|
| `$x = $a / $b catch (ArithmeticError) => 0` | `$x = (($a / $b) catch … => 0)` |
| `$x = $m["k"] ?? f() catch (IOError) => ""` | `$x = (($m["k"] ?? f()) catch … => "")` |
| `f() catch (A) => $y ?: 1 catch (B) => 2` | `f() catch (A) => ($y ?: 1) catch (B) => 2` |
| `f() catch (A $e) => throw new B({previous: $e})` | the `throw` takes everything to its right |

A `throw` arm is therefore written last or parenthesised, and an assignment inside an arm is
parenthesised as it is inside a ternary arm.

Two parses this rules out by construction: `try` never appears in the expression form, because the
guard starts at the expression rather than at a keyword; and a `catch` after an expression can only be
this form, because a block `catch` follows a `}` the statement parser is already inside.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/pipeline-and-catch/#catch-expression" title="An expression-level catch is a chain of typed arms guarding one expression"><code>expressions/catch-expression</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#catch-arm-is-an-expression" title="An arm body is an expression, which admits throw and refuses return by name"><code>expressions/catch-arm-is-an-expression</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-precedence" title="|&gt; binds tighter than every binary operator and looser than unary"><code>expressions/pipeline-precedence</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0119.md">record 0119</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-syntax/src/parser/tests/expr.rs"><code>crates/nvs-syntax/src/parser/tests/expr.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="catch-arm-is-an-expression">

## An arm body is an expression, which admits `throw` and refuses `return` by name

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#catch-arm-is-an-expression"><code>expressions/catch-arm-is-an-expression</code></a>
</div>

The arm body is parsed as an expression, and that is the entire rule for what it may hold.

**`throw` is allowed**, because `throw expr` is already an expression. `f() catch (IOError $e) =>
throw new IOError("count failed", {previous: $e})` wraps and rethrows in one line, and the variable on
the left is never assigned — the same answer `$x = $y ?? throw new …` already gives. A `throw` arm is
typed `never` and so contributes nothing to [`expressions/catch-result-type`](/docs/rules/expressions/pipeline-and-catch/#catch-result-type "The result type is the union of the guard and every arm, and an arm is checked from the pre-guard state")'s union.

**`return`, `break` and `continue` are refused**, because they are statements. The parser names them
rather than reporting a generic expected-expression: `E0126` — *an arm is an expression; `throw` is
one, `return` is not; for an early return write the block form.*

The refusal is deliberate and not a gap to fill later. A `return null` on an `IOError` hides a
failure, and the block form's five lines are the right price for choosing that.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/pipeline-and-catch/#catch-expression" title="An expression-level catch is a chain of typed arms guarding one expression"><code>expressions/catch-expression</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#catch-result-type" title="The result type is the union of the guard and every arm, and an arm is checked from the pre-guard state"><code>expressions/catch-result-type</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#bare-throwable-arm-warns" title="An unbound catch (Throwable) arm that supplies a value is warned about"><code>expressions/bare-throwable-arm-warns</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0119.md">record 0119</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-return-in-a-catch-arm-is-refused-naming-the-block-form.nvst"><code>tests/conformance/reject/a-return-in-a-catch-arm-is-refused-naming-the-block-form.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-throw-arm-leaves-through-the-enclosing-finally.nvst"><code>tests/conformance/lang/a-throw-arm-leaves-through-the-enclosing-finally.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/catch_expression.rs"><code>crates/nvs-types/tests/catch_expression.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="catch-result-type">

## The result type is the union of the guard and every arm, and an arm is checked from the pre-guard state

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#catch-result-type"><code>expressions/catch-result-type</code></a>
</div>

The expression's type is the union of the guarded expression's type and every arm's, computed by the
same union the `match` arms use. `int $rows = $db->count($q) catch (IOError) => 0;` is `int`; `catch
(IOError) => null` makes it `?int`, and a declaration on the left refuses that with the mismatch
diagnostic it already has. The arm is checked against nothing but the position the whole expression
sits in.

An arm is checked from the **pre-guard** definite-assignment state, as a block clause is: a local the
guarded expression assigned cannot be assumed assigned inside the arm, because the arm runs precisely
when the guard did not complete.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/pipeline-and-catch/#catch-expression" title="An expression-level catch is a chain of typed arms guarding one expression"><code>expressions/catch-expression</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#catch-arm-is-an-expression" title="An arm body is an expression, which admits throw and refuses return by name"><code>expressions/catch-arm-is-an-expression</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#nullable-conversion" title="expr as ?T yields the converted value or null, and never throws"><code>expressions/nullable-conversion</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0119.md">record 0119</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/catch_expression.rs"><code>crates/nvs-types/tests/catch_expression.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="bare-throwable-arm-warns">

## An unbound `catch (Throwable)` arm that supplies a value is warned about

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#bare-throwable-arm-warns"><code>expressions/bare-throwable-arm-warns</code></a>
</div>

`W1006` warns on an expression-level arm that names `Throwable`, **binds no variable**, and whose body
is not a `throw`.

In the block form a `catch (Throwable)` has a body with room to log or re-raise. In the expression
form the body *is* the value, so an unbound arm over the root of the exception tree is by construction
*discard every failure, including the ones this site never anticipated* — the suppression operator
this language removed, regrown as a one-liner.

The warning names the two honest spellings: name the class the site expects, or bind `$e` and carry
it. It is a warning rather than an error because the hazard is a habit and not a type error, and
`nvs check` is where habits are named.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP's <code>@</code> is gone, and the one-liner that would regrow it is named at compile time instead</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/pipeline-and-catch/#catch-arm-is-an-expression" title="An arm body is an expression, which admits throw and refuses return by name"><code>expressions/catch-arm-is-an-expression</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#catch-expression" title="An expression-level catch is a chain of typed arms guarding one expression"><code>expressions/catch-expression</code></a> <a href="/docs/rules/expressions/truthiness-and-equality/#nullable-condition-lint" title="A ?T used directly as a condition is warned about, not refused"><code>expressions/nullable-condition-lint</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0119.md">record 0119</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0066.md">record 0066</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/catch_expression.rs"><code>crates/nvs-types/tests/catch_expression.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="catch-lowers-to-block-form">

## The expression form lowers to the block form's landing pad, with the value in a temporary

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#catch-lowers-to-block-form"><code>expressions/catch-lowers-to-block-form</code></a>
</div>

There is no run-time representation of the expression form. It lowers to the block form's lowering —
push the protected region, lower a body, take the thrown reference in the handler block, dispatch by
a chain of class tests, re-raise what no arm matched, join — differing in exactly one way: the body
and each arm produce a value, which is written to one temporary and joined by a phi, the way `match`
joins its arms.

Nothing in the runtime or in codegen changes, and the landing pad is the one already emitted
([`errors/propagation`](/docs/rules/errors/how-an-error-travels/#propagation "An error propagates as a checked return, never by unwinding")). The cost of a guarded expression that does not throw is the block form's:
zero on the happy path ([`errors/throw-is-not-slower`](/docs/rules/errors/how-an-error-travels/#throw-is-not-slower "A throw costs no more than a return, and only its raise allocates")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/pipeline-and-catch/#catch-expression" title="An expression-level catch is a chain of typed arms guarding one expression"><code>expressions/catch-expression</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#catch-result-type" title="The result type is the union of the guard and every arm, and an arm is checked from the pre-guard state"><code>expressions/catch-result-type</code></a> <a href="/docs/rules/errors/how-an-error-travels/#propagation" title="An error propagates as a checked return, never by unwinding"><code>errors/propagation</code></a> <a href="/docs/rules/errors/how-an-error-travels/#throw-is-not-slower" title="A throw costs no more than a return, and only its raise allocates"><code>errors/throw-is-not-slower</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0119.md">record 0119</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0002.md">record 0002</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/catch-arms-are-tried-in-order-and-the-rest-is-rethrown.nvst"><code>tests/conformance/lang/catch-arms-are-tried-in-order-and-the-rest-is-rethrown.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-throw-arm-leaves-through-the-enclosing-finally.nvst"><code>tests/conformance/lang/a-throw-arm-leaves-through-the-enclosing-finally.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-ir/src/lower/exception.rs"><code>crates/nvs-ir/src/lower/exception.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="first-class-callable-syntax">

## `Name(...)` is the only spelling that takes a reference to a declared member

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#first-class-callable-syntax"><code>expressions/first-class-callable-syntax</code></a>
</div>

`Name(...)` is the only way to take a reference to a declared method or function member:
`Core\Str::length(...)`, `$user->getName(...)`, `self::helper(...)` (early-bound) and
`static::helper(...)` (late-bound). There is no second reference-taking spelling — no `::ref` form,
and no reuse of `::class`, which stays a class-name-to-string operator unrelated to producing a
callable value.

Every spelling above resolves a member at the reference itself rather than at call time, and the
checker records that resolved target under a variant that cannot be mistaken for an ordinary call.
Two shapes are refused for having no member to resolve: `new C(...)`, because `new` names a class and
this syntax builds a closure carrying a callee rather than an allocation — write `fn (): C => new C(…)`,
which also says which arguments the construction takes; and `$m->method(...)` on a `mixed` receiver,
because a closure value outlives the site and there is no class present to read a callee off.

A `callable` may carry its signature. The resolvability this rule is after comes from the reference at
the value's creation site rather than from the static type, so it holds whether or not the slot being
filled declares one.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-substitution" title="|&gt; substitutes one hole at parse time, and has no run-time representation"><code>expressions/pipeline-substitution</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#nullable-conversion-availability" title="as ?T is refused where the conversion cannot fail, where none exists, and at every class target"><code>expressions/nullable-conversion-availability</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0027.md">record 0027</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0015.md">record 0015</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0031.md">record 0031</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0136.md">record 0136</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-first-class-callable-names-a-member-not-a-class.nvst"><code>tests/conformance/lang/a-first-class-callable-names-a-member-not-a-class.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-first-class-callable-forwards-every-argument.nvst"><code>tests/conformance/reject/a-first-class-callable-forwards-every-argument.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/callable.rs"><code>crates/nvs-types/tests/callable.rs</code></a></dd></div></dl>

</div>
