A debug record reaches the service by being written to a file the writing core owns alone, never over
a socket, and the service reads a spool file only once it has been sealed.

The application therefore opens no listener and makes no outbound connection for this: there is
nothing on the request path but a buffered append to an already-open descriptor, which is cheaper
than a socket write rather than more expensive. It is never `fsync`ed per record — that, and only
that, is what would make this cost milliseconds. Nothing is lost while the service is stopped,
because the files are simply still there when it starts.

**One file per writer, sealed by an atomic rename** at 4 MiB or 250 milliseconds after its first
record, whichever comes first. A writer is one core's thread under `nvs serve` and the one thread of
`nvs run` or `nvs test`, so it needs no lock. Per writer because the server accepts per core, and
`O_APPEND` gives an atomic offset bump and not an atomic large write, so a shared file would tear
records that a dumped object tree easily makes large enough to tear. Sealed by rename because a reader
then never has to ask whether a file is still being written. A writer with no record opens no file and
arms no timer, so an idle server costs nothing here. Deleted after ingest, so this stream needs no
rotation of its own. An unsealed file untouched for a minute is past any live writer's seal, and the
ingester reads it as sealed.

The seal interval is the latency between a dump and its appearing, which is what buys everything
above.

**Where it lives.** `debug/spool/` in the data folder, `<writer>.<n>.open` while written and
`<writer>.<n>.spool` once sealed. `<writer>` is the process id, the process start time and the thread's
index, so a restarted process never reuses a name and the writer id with the line's sequence number is
a dedupe key (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`).

**What a line is.** One JSON object per line, with a `kind`: `request` (the request's outcome and,
in `development`, its input and its response,
`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`),
`event` (a trace event, `rule:observability/trace-events-carry-a-kind`, with a `query` event's call
site id where the compiler passed one), `dump` (a `Core\Debug::dump` record's bounded tree,
`rule:errors/record-transformations`), `log` (a log record of the request, because a development
server logs to its terminal and the viewer has no file to read) and `dropped` (how many records the cap
evicted). Every line but `dropped` carries the trace id, so the viewer joins them. A line is at most
4 MiB; a record that would be longer is written with its tree, its fields or its bodies elided to fit,
and the elision says so. One that is still too long is counted in the next `dropped` line.

Every line has `writer`, `seq` (its number among the writer's lines, from 0), `at_us` (when it was
written, in Unix microseconds), `kind`, and `trace` (32 hex digits) on every kind but `dropped`. The
rest is the kind's. A field marked optional is left out when it has no value, and a reader takes a
missing optional field as absent, so a field added later still reads an older spool.

| `kind` | Fields |
|---|---|
| `request` | `start_us`, `duration_us`, `method`, `path` without the query, `status`; optional `route`, `exception` (`class`, `message`, `file`, `line`), `input` (`query`, `headers` as name and value pairs, `body`) and `response` (`headers`, `body`) |
| `event` | `event` (`call`, `gc`, `spawn`, `query` or `http`), `label`, `start_us`, `end_us`; optional `status` and `query` (`driver`, `sql`, optional `rows` and `site`) |
| `dump` | `file`, `line`, `tree` (the dump's JSON, `null` when elided), `elided` |
| `log` | `level`, `message`, `fields` (an object, empty when elided), `elided` |
| `dropped` | `records` (lost since the last `dropped` line), `files` (deleted by the cap) |

A `body` has `read`, `cut`, and optional `declared` (the length the request declared), `text` (the
bytes, when they are UTF-8) and `base64` (the bytes, when they are not). `nvs_runtime::spool::Line` is
the one type that writes and reads a line, and `crates/nvs-cli/tests/fixtures/debug/one-of-each.spool`
is one line of each kind as the writer writes it.

**The records are at rest, so the spool is guarded like a secret.** The spool and its directory are
created for the service's account alone, in a directory that must pass the same ownership check a
configuration file gets (`rule:config/ownership-is-the-trust-boundary`). A byte cap with oldest-first
eviction is mandatory rather than advisory, because nothing drains the spool while the service is
stopped. `[debug] spool_max`, a `System` key and 256 MiB by default, bounds one process's spool, split evenly between its
writers, and a writer past its share deletes its own oldest sealed file and writes a `dropped` line
with the count. A writer never reads another writer's files, so the cap needs no directory scan.
Because the files are the transport, a container needs none: the developer already mounts the project
directory, and the same property makes the service a viewer for a copied-down spool or a CI artifact.

- **Memory:** one write buffer per writer, held while the writer has an open file, and no queue
  anywhere. **Disk:** at most `spool_max` per process.

**Not on disk.** `nvs_runtime::spool` is the writer, and nothing calls it yet: no request writes a
record, and nothing reads a spool file. `nvs_runtime::spool::seal_due` is not yet called from a core's
loop, so until it is, an open file is sealed by its writer's next record or read by the ingester
once it is a minute old.

**Declined:**
- **A socket collector with the application as its client.** It needs a bounded queue, a background
  drain, a circuit breaker, a retry window, a drop counter, a connect timeout on the request path and
  its own authentication. Every one of those exists only to survive the collector being absent or
  slow, and a file is absent-proof by construction.
- **One file per record.** It costs a create, a write, a close and a later delete per record. On
  Windows real-time antivirus scans each new file on close, and an undrained directory of small files
  takes seconds to enumerate.
- **A cap enforced across every writer's files.** Each seal would scan the directory, and the writers
  would need to agree on which file is oldest.
