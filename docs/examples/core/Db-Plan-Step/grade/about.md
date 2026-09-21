Tells you what one step of a schema plan can cost, at worst.

`Core\Db\Plan\Step::grade` returns a `Core\Db\Plan\Grade` case, and there are three of them. `Safe`
cannot lose data, cannot fail on rows that are already there, and cannot hold a long lock. `Locking`
cannot lose data, but it can fail on existing rows or block writes for a long time. `Destructive`
can lose data. Every drop is `Destructive`, and so is any change SQLite has to make by copying the
whole table.

The grade is the worst case, not the likely one. A change nobody wrote a rule for is graded up,
because over-reporting a risk costs one confirmation and under-reporting it costs an outage.

This is what lets a program run part of a plan on its own. `Core\Db\Schema::applySafe` runs a plan
only when every step in it is `Safe`, and the same decision written by hand is a comparison with
`==`.

The examples read the grade of each step, decide whether a deployment may run unattended, and hold
the risky steps back for a maintenance window.
