A generated decoder does not stop at the first bad field. It decodes every field, **accumulating
issues**, and if any were recorded it throws once, before the constructor is called, so no half-built
object exists.

An issue is `{path: string, message: string}`, and `path` is a dotted path into the payload —
`"address.city"`, `"tags.3"` — so a nested class's issues arrive at the top-level `catch` already
located. For a row decode it is the column name. `ParseError` carries the list; no new exception
class was added for it, because `ParseError` is already exactly the right node: *input did not match
a format this code declared*.

A whole list is the point: a submitted form has to tell its user about every bad field at once, not
one per round trip.

`Core\Db\DbError` has no `issues` slot of its own yet, so the row half's per-field refusals are
reported on a `ParseError` instead. Gap 4 in `crates/nvs-stdlib/src/db/mod.rs` is where that is
recorded.

- **Memory:** the accumulator is allocated only when the first issue is recorded, so the successful
  path — every request that is not an error — allocates nothing for it.

**Declined:**
- **Reporting the first failed field only.** It suits a configuration file and fails a form, whose
  user would be told about one mistake at a time.
- **A new exception class per format.** The closed `Throwable` set exists so that `catch` is
  learnable (`rule:errors/throwable-hierarchy`), and what was missing was a field on an existing
  class, not a new class.
