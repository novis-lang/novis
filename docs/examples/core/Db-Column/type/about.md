Says what kind of value a column holds, as one of the `Core\Db\ColumnType` cases.

A query result describes its columns before you read any rows. `type()` returns the case for one
column: `Int`, `Text`, `Float`, `Date` and eleven more. The case comes from what the column was
declared as in the table. It does not come from the value in any one row.

Every database gives you the same set of cases, so your program can compare against
`Core\Db\ColumnType::Int` instead of against a name such as `int4`, `INTEGER` or `bigint`. Those are
three databases' words for the same thing.

**Good to know:** a column the query works out, such as `price * 2`, was never declared anywhere, so
its case is `Other`.
