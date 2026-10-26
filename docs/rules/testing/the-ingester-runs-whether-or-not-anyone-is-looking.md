The ingester owns every write to the index and runs from the moment the service starts, whether or
not a browser has ever connected; the viewer is a reader that never opens a log file, parses a line
or scans a directory.

That split is both the performance argument and the correctness one. The viewer's worst case is a
bounded indexed query, so no enormous or corrupt file can reach the UI; and draining the spool is not
an effect of somebody having a tab open. Putting the ingester inside the application process is
refused: it would put directory scanning and index writes on the request path, in every deployment,
to save a hop that costs nothing.

**Two ingest modes.** A spool file is read to its end, inserted, and deleted. A log file named with
`nvs debug --tail <file>` is read from a checkpoint, inserted, and never touched — the checkpoint keyed
on the file's identity plus its offset plus a hash of its first line, because a path misses a rotation
and an identity alone misses identity reuse and truncation in place. Every record carries a **dedupe
key** under a unique index and inserts ignoring conflicts, so re-ingesting anything is harmless: the
writer's id and sequence for a spool record, the file identity and byte offset for a tailed one.

A tailed file is read in the `json` log format, and a line in any other is an `unparseable` record.
Its last line is read once it has a newline, since the writer may be partway through it. The
checkpoint and the rows it moves past are committed in one transaction. When the file at the path is
no longer the checkpoint's, the rest of `<file>.1`, the name a Novis log is rotated to, is read
first if that file is the checkpoint's, so the lines written just before a rotation are kept; the
new file is then read from its start. `crates/nvs-cli/src/debug/tail.rs` is the tailer.

The ingester lists the spool directory every 250 milliseconds, the seal age, while files arrive, and
every two seconds once none has arrived for a minute. A listing of an empty directory is the whole cost
of an idle service.

The order is read, insert, commit, **then** delete. A crash in that window re-ingests, which the
dedupe key makes a no-op. **The index is a cache of what was recorded**: losing it loses history and
never correctness, which is what licenses running it in its fastest mode — write-ahead logging, so the
one writer and the readers do not block each other, and synchronous writes off — and makes a corrupt
index a file to discard rather than a failure to resolve. The documented risk of that mode is
corruption rather than lost transactions, so the viewer offers discarding the index as an action, and a
tailed file is then read again from its start.

**The viewer is pushed a watermark, not records** — a monotonic sequence published after each
committed batch, with the browser fetching the delta through the same query path it uses for
everything else. One rendering path serves live and historical data, a backgrounded tab cannot make
the ingester buffer, and a browser that was closed needs no replay buffer. That sequence, not a
record's own timestamp, orders the live view, which makes it immune to clock skew between writers and
to a garbage timestamp in a corrupt file.

- **Memory:** one line at a time, of at most 4 MiB, and one open transaction per file, bounded by
  `rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`'s work per source.

The ingester is `nvs debug` (`crates/nvs-cli/src/debug.rs`): it raises the watermark in the index's
`meta` table with each batch. The viewer, its push of the watermark and its action that discards the
index are `rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`'s.
