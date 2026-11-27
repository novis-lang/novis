The expression's type is the union of the guarded expression's type and every arm's, computed by the
same union the `match` arms use. `int $rows = $db->count($q) catch (IOError) => 0;` is `int`; `catch
(IOError) => null` makes it `?int`, and a declaration on the left refuses that with the mismatch
diagnostic it already has. The arm is checked against nothing but the position the whole expression
sits in.

A guarded expression or an arm body that is a `void` call is `E1020` where it is written, whether the
expression is a value or a statement: that part has no value for the union or the join, as a `match`
arm has none (`rule:expressions/a-match-has-at-least-one-arm`). A `void` call guarded as a statement
is the block form, `try { save(); } catch (IOError $e) {}`. The refused part counts as an error type,
so the position the expression sits in reports nothing more.

An arm is checked from the **pre-guard** definite-assignment state, as a block clause is: a local the
guarded expression assigned cannot be assumed assigned inside the arm, because the arm runs precisely
when the guard did not complete.
