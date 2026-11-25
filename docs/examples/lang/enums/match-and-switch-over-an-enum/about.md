Chooses what to do for each case of an enum.

`match` and `switch` both compare the subject with each case label, from the first one written to
the last. `match` is an expression: it returns the value of the arm that fits, and nothing runs after
that arm. A `match` over an enum must handle every case. Write an arm for each case, or a `default`
arm for the cases you do not name. If a case is missing, the program does not compile.

`switch` runs statements instead of returning a value. An arm ends at its `break`. An arm without a
`break` runs on into the arm written below it, which is useful when one case needs everything a
later case does and a little more.

**The examples below** show a `match` with a `default` arm, then the error a `match` over a `mixed`
value throws when no arm fits, then a `switch` whose first arm runs on into the second, and last a
`match` that misses a case.
