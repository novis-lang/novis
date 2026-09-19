Chooses what to do for each case of an enum.

`match` and `switch` both compare the subject with each case label, from the first one written to
the last. `match` is an expression: it returns the value of the arm that fits, and nothing runs after
that arm. When no arm fits and there is no `default`, `match` throws an error, so a `default` arm is
how you cover the cases you did not name.

`switch` runs statements instead of returning a value. An arm ends at its `break`. An arm without a
`break` runs on into the arm written below it, which is useful when one case needs everything a
later case does and a little more.

**The examples below** show a `match` with a `default` arm, then the error a `match` throws without
one, then a `switch` whose first arm runs on into the second.
