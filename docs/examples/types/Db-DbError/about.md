The error a database raises when it refuses a statement.

A key that is already taken, a column left empty, a row another table still points at, a lock two
requests are both waiting on: every refusal the server makes arrives as this one error, whichever
database is answering. It carries the reason as a `kind` drawn from one closed list that reads the
same on all of them, so code written against PostgreSQL recognises the same refusal from MySQL or
SQLite. It also carries the statement the program wrote, so one place can log both.

That is what makes a refusal something a program can act on rather than only report: a duplicate
address is the person's to fix and worth a sentence they understand, a deadlock is worth trying
again, and anything else is worth passing on untouched.

**Good to know:** a mistake in the *call* — a value the driver has no form for, a second statement
while rows are still being read — is a `LogicError` instead. This error is only ever the server's
own answer.
