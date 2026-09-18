What one step of a schema change can cost you, at worst.

A schema plan is the list of steps that would bring a database in line with the schema your code
declares, and every step carries one of these grades. `Safe` means the step cannot lose data, cannot
fail because of rows that are already there, and cannot hold a long lock — that is the half a
deployment can run by itself, at any hour, with nobody watching. `Locking` keeps your data but can
block writes for a long time, or fail on the rows you already have. `Destructive` can lose data, and
every drop is in there.

**In plain words:** the grade is the worst that could happen, not the likely one. A step nothing has
a rule for is graded up rather than down, because over-stating the risk costs one confirmation and
under-stating it costs an outage.
