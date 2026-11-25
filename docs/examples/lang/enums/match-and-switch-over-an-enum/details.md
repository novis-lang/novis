A `match` over an enum must handle every case. When the subject has an enum type such as `Mode`,
and the `match` has no `default` arm, each case needs an arm. A missing case does not compile
(`E1016`), and the message names each missing case. For a subject of type `?Mode`, an arm must also
handle `null`. In the editor, the quick fix for `E1016` adds one arm for each missing case.

An arm covers a case when its label is that case, or a class constant whose value is that case. Any
other label covers no case, so that `match` needs a `default` arm.

A `match` over a `mixed` value is not checked, because the compiler does not know which enum the
value belongs to. When no arm fits and there is no `default`, it throws a `LogicError` when the
program runs.

A label must be a case of the subject's enum. A case of another enum is the error `E0466`, and so is
a number such as `1`. This is the same in a `match` arm and in a `switch` `case`. A `null` label is
`E0466` too, unless the subject has a type such as `?Mode` that allows `null`.

An arm for `Mode::Read` does not narrow the subject. Inside that arm, `$m` still has the type `Mode`.
Passing it to a parameter of the type `Mode::Read` is the error `E0401`. The same is true inside a
`switch` `case`. Write `match (true)` with the arm `$m == Mode::Read` to narrow `$m` in that arm.

A `switch` does not need a `case` for every case of the enum. A `switch` with no matching `case` and
no `default` runs none of its code. It does not throw.

When the subject has a smaller type such as `Mode::Read|Mode::Write`, an arm for `Mode::Exec` still
compiles. That arm never runs.

The labels are still cases when the subject is `mixed`. The `match` compares each label by its number. A `mixed` variable that contains the
number `1` matches the arm `Mode::Write` when `Write` is the case with the number `1`.
