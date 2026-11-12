What each `Core` member needs is declared once, in one file, as two lists that together cover every
class. The capability table has a class, a member, and an optional capability, with one row for
each capability a member can ask. The second list names each class none of whose members reaches a
door, with one reason per class. A third table names each language construct that reaches a door,
such as `spawn script`, and the capability it needs. **None of them is read at run time.** The
checker reads them while compiling, to hold each call to its namespace's grants
(`rule:security/authority-is-the-enclosing-namespace`), the completeness tests read them, and the
metadata command renders each as a roster of its own, which is how any renderer that wants a
member's capability beside its card gets one (`rule:tooling/one-json-several-renderers`; the join is
on `(class, member)` at render time). The operator's grant is enforced at the doors
(`rule:security/capability-check-at-the-door`), which do not consult them, so no edit to a list
widens what the operator granted. A row that understated its door would widen what a namespace
compiles, which is why `rule:testing/capability-completeness-test` holds every door to its rows and
every class on the second list to no door.

A field on each member row was the obvious shape and is rejected for two reasons, in this order.
**"What can this runtime do to my machine" should be one screen of one file**; spread across dozens of
class definitions in dozens of modules it is dozens of greps and a judgement about whether you found them
all, which is precisely the question a security review is trying not to have to make. And a field that
is empty on the overwhelming majority of rows documents nothing while being maintained everywhere.

The second list is one row per class and not a row with no capability for each member of every
class. Thousands of equal rows would hide the few that matter, and a class with one reason states
it once.

The locality it gives up is bought back mechanically: a test fails on an entry naming a class or
member that does not exist, another fails on a class on neither list, and another on a member of a
capability-bearing class that owes an entry and has none.

**Declined:**
- **A row with no capability for every member of every class.** It hides the rows that name one.
- **An exception list in the test.** `rule:testing/capability-completeness-test` forbids it.
- **A compile-time assertion over both lists.** It needs string compares in a `const fn`, for no gain
  over a test the gate runs.
