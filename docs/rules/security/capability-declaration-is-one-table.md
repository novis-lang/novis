What each `Core` member needs is declared once, in one table: a class, a member, and an optional
capability. **The table is never read at run time.** It is audit data — the completeness test reads it, and
the metadata command renders it as a roster of its own, which is how any renderer that wants a
member's capability beside its card gets one (`rule:tooling/one-json-several-renderers`; the join is
on `(class, member)` at render time). Enforcement is the doors
(`rule:security/capability-check-at-the-door`), which do not consult it, so the table cannot be the
thing an attacker edits to gain a permission.

A field on each member row was the obvious shape and is rejected for two reasons, in this order.
**"What can this runtime do to my machine" should be one screen of one file**; spread across dozens of
class definitions in dozens of modules it is dozens of greps and a judgement about whether you found them
all, which is precisely the question a security review is trying not to have to make. And a field that
is empty on the overwhelming majority of rows documents nothing while being maintained everywhere.

The locality it gives up is bought back mechanically: a test fails on an entry naming a class or
member that does not exist, and another fails on a member that owes an entry and has none.
