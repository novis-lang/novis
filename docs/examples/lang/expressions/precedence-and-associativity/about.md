Precedence decides which operator runs first. Associativity decides the order when two operators of
the same strength meet.

Novis uses the order most languages use: `*` and `/` before `+` and `-`, then the comparisons, then
`&&`, then `||`, and the assignment last. `**` runs before a minus sign in front of it, and a chain
of them is read from the right. So `-2 ** 2` is `-4` and `2 ** 3 ** 2` is `512`. A conversion with
`as` binds tighter than every operator around it.

**Good to know:** the compiler gives warning `W1022`
for two mixes that are easy to misread. In `$n ?? 0 + 10`, the `+` runs first, so the default is
`10`.

**The examples below** show the arithmetic order, `**` with a minus sign, and `as` and `.` in a line
of text. The last one shows `!` with `is`, the pipeline `|>` and a method call on `new`.
