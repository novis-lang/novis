A `match` with no arms is `E0476` where it is written. PHP parses one and throws `UnhandledMatchError` on
every evaluation, so no program that ran is lost. A `match` is an expression, and one whose every path
throws has nothing for the position it sits in to bind, pass or return, and no value for a merge with
no incoming edge to carry. A written arm, or `default => throw …`, says the same thing on purpose.

For the same reason an arm whose body is a `void` call is `E0868` where it is written, whether the
`match` is a value or a statement: the arm has no value for the merge to carry. A statement per case
is an `if` or a `switch`. A `void` call stored at `mixed` is `E0401`, as it is at any other declared
type, because `mixed` takes every value and a `void` call has none.
