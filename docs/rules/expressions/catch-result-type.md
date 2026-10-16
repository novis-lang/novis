The expression's type is the union of the guarded expression's type and every arm's, computed by the
same union the `match` arms use. `int $rows = $db->count($q) catch (IOError) => 0;` is `int`; `catch
(IOError) => null` makes it `?int`, and a declaration on the left refuses that with the mismatch
diagnostic it already has. The arm is checked against nothing but the position the whole expression
sits in.

An arm is checked from the **pre-guard** definite-assignment state, as a block clause is: a local the
guarded expression assigned cannot be assumed assigned inside the arm, because the arm runs precisely
when the guard did not complete.
