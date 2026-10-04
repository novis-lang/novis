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
