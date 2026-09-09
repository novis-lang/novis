A malformed line becomes a record that says so and an unreadable file is quarantined under a backoff,
so nothing on disk can stop the ingester.

Line length is capped, and past the cap the line is truncated, marked, and skipped to the next
newline. A corrupt file is often one enormous line with no newline in it, and reading it whole is how
a log tailer dies.

A line that will not parse is kept as an `unparseable` record carrying its file, its offset and its
truncated bytes, and is shown. Dropping it silently would leave a viewer quietly disagreeing with
what is on disk, which is worse than an ugly row. Invalid UTF-8 is a lossy conversion with a flag,
never an error.

A file that fails at the file level — permissions, an IO error, a mount that went away — is
quarantined with its error and a retry-after, never retried hot, and listed in the UI, so a file that
stopped being read says so instead of merely not appearing. Other files keep flowing.

Work is bounded per source per pass and sources are taken in turn, so one enormous file cannot starve
the live spool. A disk that is full or an index that errors backs off rather than spinning, and never
deletes a spool file whose batch did not commit.
