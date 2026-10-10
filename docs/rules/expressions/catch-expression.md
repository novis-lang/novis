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

**Why the form exists.** A failed conversion, an absent key, every `Core` failure, an arithmetic
overflow and a division by zero all throw, and each costs a caller with a fallback a five-line block.
Definite assignment makes that block dearer than it looks: a local assigned inside `try` is readable
after it only when every `catch` assigns it too, so the declaration, the two assignments and the read
are kept in step by hand. An expression assigns on every path it completes, so the declaration and
the fallback are one line and the checker has nothing to track.

The form sits beside the two value-shaped spellings and does not replace them: `$s as ?int ?? 1` is
the conversion (`rule:expressions/nullable-conversion`) and `$a["k"] ?? $d` the absent key. `$s as int
catch (…) => null` is legal and pointless, and no diagnostic names it. What a program could catch or
fail to catch does not change: the same landing pad, the same dispatch and the same re-raise run
(`rule:expressions/catch-lowers-to-block-form`).

**Declined:**
- **Extending `??` to swallow any throw.** It is untyped, discards the thrown object and reads as a
  null check. Every reason a conversion throws on garbage would be one `??` away from undone.
- **`try? expr` yielding `?T`.** It is untyped, so it catches everything, and it conflates *threw*
  with *returned `null`*. It is redundant with `as ?T` for the case it does best, and lands every
  result in the nullable-condition trap (`rule:expressions/nullable-condition-lint`).
- **A bare fallback with no `=>`**, as in `f() catch (IOError) 0`. The value dangles after the
  parenthesis with nothing to say it is the result, and `match` already owns the `condition => value`
  syntax a reader knows.
- **A braced multi-arm form**, as in `f() catch { IOError => 0, ParseError $e => throw … }`. Chained
  arms already give several classes, so it would be a second syntax for the same thing.
- **A comma-ok pair** (`v, err := f()`). It needs a tuple return Novis does not have, and costs three
  lines per call.
- **A `Result` type with a `?` operator.** A sum type is a language-scale decision, and it is declined
  for conversions on the same grounds.
- **A `try` keyword opening the guard**, as in `try f() catch …`. The token carries no information:
  the guard starts where the expression does, and an unguarded throw already propagates without one.
