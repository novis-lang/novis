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
(`rule:errors/throwable-hierarchy`). A union is refused with the block form's own wording: *write two
arms*. The binding is optional and, where written, is a local of the enclosing function under the
block form's rule.

**Arms are clauses of one guard, not guards of each other.** `f() catch (A) => x catch (B) => y` tries
`A` then `B` against what `f()` threw, and `x` is not guarded by the `B` arm. A supertype written
first shadows the arms after it, as in the block form.

The block form is unchanged in every particular and remains the only spelling with `finally`. Resource
cleanup stays there; this form has no one-line twin for it.
