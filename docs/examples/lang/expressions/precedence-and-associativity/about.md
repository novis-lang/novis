Precedence decides which operator in an expression runs first, and associativity decides the order
when two operators of the same strength meet.

Novis uses the order most languages use: `*` and `/` before `+` and `-`, then the comparisons, then
`&&`, then `||`, and the assignment last. Four rows are worth learning, because they are the ones
people check. `**` runs before a minus sign written in front of it and a chain of them is read from
the right, so `-2 ** 2` is `-4` and `2 ** 3 ** 2` is `512`. A conversion with `as` binds tighter
than every operator around it. The dot that joins two strings binds looser than `+`, `-`, `*` and
the shifts. A ternary written inside another one groups to the right.

**Good to know:** parentheses cost nothing when the program runs, so write them wherever a reader
would otherwise have to look the order up. The compiler gives warning `W1022` for two mixes that are
easy to misread. In `$n ?? 0 + 10`, the `+` runs first, so the default is `10`. In
`"n=" . $ok ? "x" : "y"`, the whole `"n=" . $ok` is the condition.

**The examples below** take these in turn: the ordinary arithmetic order first, then `**` with a
minus sign, then `as` and `.` in a line of text built from a form field.
