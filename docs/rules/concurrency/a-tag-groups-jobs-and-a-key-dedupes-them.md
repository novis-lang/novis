`push`'s `tag` names a group of jobs and `key` admits at most one pending job, and they are two columns
because they are opposites at the point they touch. A tag exists to name **many** rows — a batch, a
tenant, an upload — so that something can later be said about all of them at once. A key exists to
admit **one**, enforced by the unique index over the released-when-claimed column
(`rule:core-classes/queue-storage-is-a-table`). Folding the two would cap every group at one pending
job, silently, at the enqueue that created it.

A tag is inert on the request path. Nothing claims on it, nothing dedupes on it, and no statement a
worker runs reads it; it is written by `push`, indexed with its queue, and read by
`rule:concurrency/queue-deletion-is-explicit-and-bounded`'s `purge` alone. That is what keeps it a
column rather than a feature: a deployment that never purges pays one nullable column and one index
write per enqueue for it, and nothing else.

**Grouping is decided at enqueue, not at removal.** A caller that wants a batch deletable tags it when
it creates the batch, because nothing can group rows that were never grouped. That is a real cost to
the caller, and it puts the grouping decision in the source at the `push` that creates the group
rather than hiding it in the sweep that removes it.

**Declined:**
- **A predicate over the payload.** `args` is one JSON document in a text column, so selecting inside
  it is a different unindexed dialect on each of `rule:core-classes/db-one-api`'s backends, over the one
  table in the runtime that grows without bound. A tag gives the same result in one indexed column on
  every backend.
- **`key` as the group.** It would cap every group at one pending job (above).
