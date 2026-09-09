The ingester owns every write to the index and runs from the moment the service starts, whether or
not a browser has ever connected; the viewer is a reader that never opens a log file, parses a line
or scans a directory.

That split is both the performance argument and the correctness one. The viewer's worst case is a
bounded indexed query, so no enormous or corrupt file can reach the UI; and draining the spool is not
an effect of somebody having a tab open. Putting the ingester inside the application process is
refused: it would put directory scanning and index writes on the request path, in every deployment,
to save a hop that costs nothing.

**Two ingest modes.** A spool file is read whole, inserted, and deleted. The application's own log is
read from a checkpoint, inserted, and never touched — the checkpoint keyed on the file's identity
plus its offset plus a hash of its first line, because a path misses a rotation and an identity alone
misses identity reuse and truncation in place. Every record carries a **dedupe key** under a unique
index and inserts ignoring conflicts, so re-ingesting anything is harmless: the writer's id and
sequence for a spool record, the file identity and byte offset for a tailed one.

The order is read, insert, commit, **then** delete. A crash in that window re-ingests, which the
dedupe key makes a no-op. The index holds nothing that cannot be rebuilt from the files, which is
what licenses running it in its fastest mode and makes a corrupt index a cache miss to discard rather
than a failure to resolve.

**The viewer is pushed a watermark, not records** — a monotonic sequence published after each
committed batch, with the browser fetching the delta through the same query path it uses for
everything else. One rendering path serves live and historical data, a backgrounded tab cannot make
the ingester buffer, and a browser that was closed needs no replay buffer. That sequence, not a
record's own timestamp, orders the live view, which makes it immune to clock skew between writers and
to a garbage timestamp in a corrupt file.
