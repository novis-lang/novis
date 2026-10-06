The error a calculation raises when it has no right answer.

Dividing by zero is the usual way to meet it. So is a remainder by zero, and a total too large for
the whole-number type it was going to be kept in. The calculation stops, so a wrong figure never
reaches a page, a database row or an invoice.

Catching it by name handles the one case a program expects — an empty basket, a missing baseline — and
leaves every other kind of failure alone. The message it carries names the operation that gave up.

**Good to know:** it sits directly under `Throwable` rather than under `RuntimeError`, so a clause
written to catch runtime errors does not catch this one.

**The examples below** show a division by a count that turns out to be zero, a total too large to hold,
and a report that names the rows it could not work out and finishes anyway.
