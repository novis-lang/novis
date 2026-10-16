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
