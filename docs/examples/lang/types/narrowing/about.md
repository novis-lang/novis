A test that proves what a value is lets you use it as that thing, inside the branch the test guards.

A value that might be missing, one that could be several different kinds of thing, or one that is one
of a few fixed words: ask about it in an `if`, and inside that branch the program reads it as whatever
the question proved. You do not convert it and you do not give it a second name — it already was that,
and now the language knows.

Four questions do this: whether a value is missing, what kind of thing it is, whether it equals one of
a closed set of values, and any of those three asked again as the arms of a `match (true)`.

**Good to know:** what was proved holds inside that branch only, and only for a plainly named value.
Writing to the value inside the branch gives the proof up, and a test on something reached through an
arrow or an index proves nothing about the next read of it — put that in a value of its own first.
