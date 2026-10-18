A `match` that has an arm for every case needs no `default`. The compiler does not check that every
case has an arm. A case with no arm throws the `LogicError` when it reaches the `match`.

A label must be a case of the subject's enum. A case of another enum is the error `E0466`, and so is
a number such as `1`. This is the same in a `match` arm and in a `switch` `case`. A `null` label is
`E0466` too, unless the subject has a type such as `?Mode` that allows `null`.

An arm for `Mode::Read` does not narrow the subject. Inside that arm, `$m` still has the type `Mode`.
Passing it to a parameter of the type `Mode::Read` is the error `E0401`. The same is true inside a
`switch` `case`. Write `match (true)` with the arm `$m == Mode::Read` to narrow `$m` in that arm.

A `switch` with no matching `case` and no `default` runs none of its code. It does not throw.

When the subject has a smaller type such as `Mode::Read|Mode::Write`, an arm for `Mode::Exec` still
compiles. That arm never runs.

In a `mixed` subject, a label is compared by its number. A `mixed` variable that contains the
number `1` matches the arm `Mode::Write` when `Write` is the case with the number `1`.
