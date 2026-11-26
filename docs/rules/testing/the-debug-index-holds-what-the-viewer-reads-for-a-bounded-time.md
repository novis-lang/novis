The index is one SQLite file, `debug/index.sqlite` in the data folder, whose tables hold every query
the viewer makes, and the ingester deletes what is older than `[debug] index_age` or past `[debug]
index_max`, the oldest request first.

| Table | One row per | What the viewer reads it for |
|---|---|---|
| `requests` | recorded request: trace id, start, duration, method, route, path, status, query, dump and log counts, its exception group, the summed duration of its events of each kind (`db_us`, `http_us`, `cache_us`, `session_us`, `file_us`, `gc_us`, `spawn_us`) and `repeats`, the most times one statement ran at one site; from goal `deeper-debug-records`, nullable `memory_peak`, `memory_limit`, `handler`, `bytes_in` and `bytes_out`; indexed on start and on `requests_route` (`method`, `route`, `start_us`) | the request list, its filters and time split, the routes view (`rule:testing/the-viewer-answers-per-route-and-per-statement`), and a request's memory, handler and sizes |
| `runs` | from goal `deeper-debug-records`, `run` line: trace id, `of`, name, start, duration, outcome, nullable memory peak and limit, its exception group, and its dump, log and query counts and summed event durations kept by the same triggers as a request's; indexed on start and on (`of`, `name`, `start_us`) | the Runs view (`rule:testing/a-run-is-recorded-like-a-request`) |
| `inputs` | request recorded with its input: query, headers, body, bytes declared, whether the body was read, and the response's headers and body | compare, replay and a test made from the request (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`) |
| `body_parts` | `body` line: trace id, request or response, offset, bytes | a body too long for its `request` line, read in offset order |
| `events` | trace event: trace id, kind, label, start, end, status, a `query` event's driver, SQL, rows, the target, and its call site's file, line and file hash; from goal `deeper-debug-records`, nullable `outcome`, the event's outcome fields as one JSON object | a request's timeline, and each event's outcome |
| `dumps` | dump: trace id, file, line, the bounded tree | a request's dumps |
| `profiles` | `profile` line: trace id, the profile file's path | a request's profile |
| `coverage` | `coverage` line: trace id, span, each file's lines and counts as JSON | the lines a request ran |
| `copies` | one copy site of a `copies` line: trace id, span, file, line, file hash, reason, copies, units; indexed on `copies_site` (file hash, line) | a request's copies, and the copies at one site |
| `logs` | log record: trace id where it has one, level, message, fields, source file and offset for a tailed line | the log view |
| `exceptions` | uncaught exception: trace id, class, message, file, line, group; from goal `deeper-debug-records`, nullable `frames` and `previous` as JSON with `frames_cut` and `previous_cut`, and a nullable `run` beside the now nullable `request`, so one names its request or its run | a request's or a run's error and its stack (`rule:testing/a-recorded-exception-keeps-its-frames-and-never-a-value`) |
| `exception_groups` | class and throw site: count, first seen, last seen | grouped exceptions |
| `search` | searchable text of one row above, with its table and row id | full-text search |
| `dropped` | `dropped` line: the records and files a writer's share of the spool cap deleted | what was lost before it was read |
| `unparseable`, `quarantine` | `rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`'s records | what was not read |
| `checkpoints`, `meta` | tailed file's checkpoint; the watermark and the schema version | the ingester itself |

**An exception group is a class and the file and line that threw it,** never the message, because a
message carries values and would split one bug into a group per value. A group's count, first seen
and last seen are updated as each exception is inserted, so the grouped view is one indexed read.

**Search is one FTS5 table** over a request's path and route, a dump's rendered text, a log message, a
query's SQL and an exception's class and message. Each row names the table and row it came from in
its rowid, the source row's id times eight plus the table's code, and a trigger on each table writes
and deletes its search rows. FTS5 is in the SQLite that `rusqlite`'s bundled build already
compiles, so search costs no new dependency. A secret is already redacted in every record before it is
written (`rule:security/redaction-reaches-the-tools-own-renderings`), so search cannot find one.

**The deletion window.** `[debug] index_age`, `7d` by default, is the oldest record kept, measured
from a request's start, and `[debug] index_max`, 1 GiB by default, is the most bytes of pages in use;
a page a deletion frees is reused, so the file stays near that size. Both are `System` keys. After
each batch the ingester deletes requests past either bound with every row that names their trace id,
in batches of a thousand so the viewer never waits on a long write. A row that names no recorded
request is deleted by its own time past `index_age`. An exception group stays while its last seen is
within `index_age`, and its count still counts deleted requests. A saved filter is the viewer's own
and is not in the index, because discarding the index must lose nothing a person wrote.

**A run is forgotten as a request is,** by its start past `index_age` and oldest first past
`index_max`, with every row that names its trace id. An event's outcome is one JSON column and not
a column per field, because the viewer draws it and never filters or sums on it, and ten kinds of
named columns would be mostly empty on every row. Each stage of goal `deeper-debug-records` that adds
a column raises the schema version once, so the index is rebuilt from the spool.

A schema version in `meta` that the running binary does not know is a cache miss: the file is
discarded and rebuilt from what is still on disk
(`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`). So is a file SQLite cannot read.

- **Disk:** at most `index_max`. **Memory:** SQLite's page cache in the service process, never in a
  server.

**Declined:**
- **Grouping exceptions by message.** One bug that puts an id in its message would show as a group per
  id.
- **Keeping the index until a person clears it.** Recorded bodies and dumps hold request data, and a
  debugging aid that keeps them forever is a store of personal data nobody chose to keep.
