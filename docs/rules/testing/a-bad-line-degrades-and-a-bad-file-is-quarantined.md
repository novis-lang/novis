A malformed line becomes a record that says so and an unreadable file is quarantined under a backoff,
so nothing on disk can stop the ingester.

Line length is capped at 4 MiB, the longest line a spool writer writes
(`rule:testing/the-debug-stream-reaches-the-service-through-the-disk`), and past the cap the line is
truncated, marked, and skipped to the next newline. A corrupt file is often one enormous line with no
newline in it, and reading it whole is how a log tailer dies.

A line that will not parse is kept as an `unparseable` record carrying its file, its offset and its
first 4 KiB, and is shown. Dropping it silently would leave a viewer quietly disagreeing with what is
on disk, which is worse than an ugly row. Invalid UTF-8 is a lossy conversion with a flag, never an
error.

A file that fails at the file level — permissions, an IO error, a mount that went away — is
quarantined with its error and a retry-after, never retried hot, and listed in the UI, so a file that
stopped being read says so instead of merely not appearing. The first retry is after one second and
each next one doubles, up to five minutes. Other files keep flowing.

Work is bounded per source per pass, at 16 MiB, and sources are taken in turn, so one enormous file
cannot starve the live spool. A disk that is full or an index that errors backs off on the same
schedule rather than spinning, and never deletes a spool file whose batch did not commit.

What that comes to in the ingester (`crates/nvs-cli/src/debug/ingest.rs`, `crates/nvs-cli/src/debug.rs`):

- **A cut line** is an `unparseable` row with its whole length beside its first 4 KiB.
- **A spool line that is not UTF-8** is an `unparseable` row with the flag, and is never inserted as a
  record: the writer writes UTF-8 alone, so such a line is damaged. A tailed log line that is not
  UTF-8 is converted and inserted as a log record with the flag, because another program may have
  written it.
- **The budget** is passed only to finish the line a pass is in. A spool file the budget stopped in
  is committed as far as it was read, is not deleted, and goes on where it stopped in the next pass.
- **A quarantined file** is a `quarantine` row: its path, its error, its failures in a row and its
  next try. The row goes once the file is read or is gone, never by age, because a file that still
  fails must still be listed. A spool directory that cannot be listed is quarantined the same way.
- **A file that cannot be deleted** after its batch committed is quarantined too: its next read adds
  nothing, under the dedupe key, and then deletes it.

- **Memory:** one 64 KiB read buffer and one line of at most 4 MiB while a file is read; one entry per
  file failing.

The viewer showing `unparseable` and `quarantine` is `rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`'s.
