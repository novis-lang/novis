A generated decoder does not stop at the first bad field. It decodes every field, **accumulating
issues**, and if any were recorded it throws once, before the constructor is called, so no half-built
object exists.

An issue is `{path: string, message: string}`, and `path` is a dotted path into the payload —
`"address.city"`, `"tags.3"` — so a nested class's issues arrive at the top-level `catch` already
located. For a row decode it is the column name. `ParseError` carries the list; no new exception
class was added for it, because `ParseError` is already exactly the right node: *input did not match
a format this code declared*.

The accumulator is allocated only when the first issue is recorded, so the successful path — every
request that is not an error — allocates nothing for it.

One thing the record describes has not landed: `Core\Db\DbError` has no `issues` slot of its own, so
the row half's per-field refusals are reported on a `ParseError` instead. Gap 4 in
`crates/nvs-stdlib/src/db/mod.rs` is where that is recorded.
