A connection released at request teardown returns to a per-core pool, and **a released connection is
reset before it is reusable**. The reset is not best-effort: a connection that cannot be proven clean
is closed, and a driver with no reset primitive is not poolable at all.

What the reset must remove is stated as a **property, not a command list**: after it, no transaction,
no temporary table, no session variable, no assumed role, no advisory lock, no listener, no open
cursor and no prepared statement the cache does not still account for. A backend added later satisfies
that property or is not pooled.

**The pool key includes every credential**, so two configuration blocks are two pools and two database
users never share a connection. It is additionally scoped to the configuration generation it was read
from: a reload can publish the same block name under a different user, and a pool keyed on the name
alone would hand the new generation's request a connection authenticated as the old one's.
