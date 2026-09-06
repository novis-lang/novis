Computing a plan is an ordinary read. It issues catalog queries through the connection a program
already holds, under the `db.connect` it already has, and nothing about it is privileged.

Applying is not. It takes the deny-by-default `db.schema` capability
(`rule:core-classes/db-capabilities`), which gates a different thing from `db.connect` and `db.open`:
not which database may be reached, but whether this program may issue DDL to it at all. It names
connection blocks, and an ungranted name throws naming the capability.

The two entry points are named for what they risk, at the call site. **`applySafe()`** refuses if any
step it would run is not `Safe`, and throws naming the first that is not.
**`applyIncludingRisky()`** says so where it is written, so a reviewer reading the call sees the
claim being made. `nvs schema plan|apply|dump` is the operator's spelling of the same thing.
