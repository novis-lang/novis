Every step of a plan carries a grade, and there are three rather than two. **`Safe`** cannot lose
data, fail on existing rows, or hold a long lock. **`Locking`** cannot lose data but can fail or
block writes for a long time — a unique index over data that already contains duplicates, `not null`
on a populated column, a type change that rewrites the table. **`Destructive`** can lose data. Two
grades would have to merge the middle into an outer one, and both merges are wrong in production:
calling a table rewrite `Safe` is how an automatic tool takes a site down without losing a byte, and
calling it `Destructive` makes the dangerous class so large that operators stop reading it.

A grade is computed by the dialect emitter, keyed on driver **and** server version, because adding a
column with a default is instant on a recent server and a full rewrite on an older one. **When the
emitter does not know the version, it grades up.** An out-of-date grader over-reports risk, which
costs a confirmation; an out-of-date optimist costs an outage. A grade is a floor, never a promise.

Each step also exposes **complete, terminated, dialect-correct SQL**, including the steps the tool
will refuse to run. The common case in a serious deployment is that the application's own credentials
cannot issue DDL at all and a DBA applies the change from a ticket — a plan whose risky steps are
elided into "3 unsafe changes" is useless to that person, and a plan they can paste is the whole
product. Emitters follow the four dialects, not the five drivers.

A dialect's spelling of a vocabulary construct is graded as that construct, not as the SQL it happens
to be: SQL Server's filtered unique index for a nullable unique key
(`rule:core-classes/a-unique-key-reads-nulls-as-distinct`) is `Locking` built over an existing table
and `Safe` inside a `CREATE TABLE`, exactly as the constraint form is on the other dialects.
