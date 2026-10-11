A job's attempts are finite with nothing configured, because an unbounded retry is an unbounded wait
wearing a different name. Between attempts the delay grows exponentially, is jittered so a fleet does
not retry in lockstep, and is capped.

A job that exhausts its attempts **moves** to the dead-letter table, carrying its payload, every
attempt's error and its timing. Every attempt's error is there because the jobs row accumulates it:
an `errors` array on the job itself gains one entry — when the attempt started, the class thrown and
its message, capped — as each attempt fails, and the move copies the array rather than writing the
last failure alone. Attempts are finite and an entry is capped, so the array is bounded by
construction. The cap exists because a thrown message is arbitrary text under the program's control,
and without it a job row would be a place to store a megabyte; it counts characters and cuts on a
character boundary, because half a code point is not text a reader can parse. The runtime never
deletes it. `stats` reports the dead-letter depth
beside the pending and claimed counts, because an unwatched dead-letter table is the classic way a
queue silently loses work and a depth nobody reads is the same as no record at all.

An attempt ceiling of zero is refused at the call rather than accepted: it asks for a job dead-lettered
by the enqueue that created it, and attempts are finite, not optional.

**Memory:** a failing job's row grows by one capped entry per failed attempt, at most attempts × (the
message cap plus the start time and the class), and the dead-letter row carries the same array. Both
are released when the row is deleted or purged, so the cost is O(in-flight work), never O(jobs run).

**Declined:**
- **A third table of attempts.** It is a second write per failure on the table the claim already
  contends over, a second thing `purge` has to delete, and a join to answer a question whose only
  reader is a person looking at one dead job.
- **Keeping only the last error.** It makes the common failure, a job that failed differently on each
  attempt, unreadable from the row.
