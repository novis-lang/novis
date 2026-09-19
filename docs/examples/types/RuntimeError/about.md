The error a program raises when something outside it said no.

A file that has gone, a document that arrived malformed, a database that gave up, a wait that ran
out: the code is right and the world did not cooperate. Novis gives that its own half of the
exception tree, with narrower names underneath — `IOError`, `ParseError`, `TimeoutError`,
`RecursionError` and the ones the database and the terminal raise — so a program can catch the whole
half in one clause or pick out the one kind it knows how to answer.

This is the half worth catching. A failure from out here is usually worth retrying, reporting or
falling back on, where the same treatment applied to a bug in the code would just repeat it.

**Good to know:** it sits *beside* `LogicError` under `Throwable`, so a `RuntimeError` clause never
takes a mistake the program made in its own call.

**The examples below** show a service raising one, one clause standing in for every narrower kind,
and a retry that knows which failures are worth a second attempt.
