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
Str::format($_, $n)` is refused for the same reason the nested call is. There is no callable allocated
and no dynamic dispatch.

The operator exists for reading order. A three-deep composition of `Core` calls reads inside-out when
it is nested, and a pipeline reads in the order it runs while every step still names the class it
calls. That class name is what a reviewer reads authority off — `Path::` cannot touch the disk, `IO::`
can — so the composition keeps the signal a fluent method would erase. A hole depends on no
argument-order convention: it reaches any argument slot, an instance method (`$_->group(1)`), an index
(`$_["email"]`), a closure in a variable and `new`. Userland classes need no annotation and no
registration, because the class is written at the call site and resolves the way it always does.

The cost is one more operator on the surface, spent on simplicity of reading, and two ways to write
one composition.

**Declined:**
- **Methods on scalars** (`$s->trim()->upper()`). They erase the class name from the call site, need
  a scalar-to-`Core` map in the type checker instead of a parser rewrite, and collide with `->` on a
  `mixed` receiver, where it already means instance access.
- **A `|>` that applies a callable at run time.** Every step would need `Class::method` as a
  value, and most `Core` methods take a trailing options argument, so they are not unary: nearly every
  step would carry a closure wrapper and read worse than the nesting it replaces. Resolving the right
  side by name at run time is a spelling `rule:expressions/method-reference` already closed. The shared
  token's habit is caught by `E0129` (`rule:expressions/pipeline-hole-once`).
- **First-argument insertion with no hole.** It cannot reach a second slot
  (`Arr::contains($haystack, $needle)`), it makes a call's arity depend on its context, and userland
  is not bound to put the subject first, so the rule would carry an exception.
- **A different token** — `~>`, `->>`, `==>`. `->>` is one glyph from `->`, `==>` is one from `=>`, and
  `~>` loses the recognition `|>` has across many languages while the lexer would still have to know
  `|>` to catch the habit.
- **A marker attribute that opts a class into fluent calls.** It would make resolution open-world and
  cost the compile-time determinism of name resolution.
