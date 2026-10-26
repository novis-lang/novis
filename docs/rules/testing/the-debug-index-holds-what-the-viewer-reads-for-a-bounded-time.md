The index is one SQLite file, `debug/index.sqlite` in the data folder, whose tables hold every query
the viewer makes, and the ingester deletes what is older than `[debug] index_age` or past `[debug]
index_max`, the oldest request first.

| Table | One row per | What the viewer reads it for |
|---|---|---|
| `requests` | recorded request: trace id, start, duration, method, route, path, status, query, dump and log counts, its exception group | the request list and its filters |
| `inputs` | request recorded with its input: query, headers, body, bytes declared, whether the body was read, and the response's headers and body | compare, replay and a test made from the request (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`) |
| `body_parts` | `body` line: trace id, request or response, offset, bytes | a body too long for its `request` line, read in offset order |
| `events` | trace event: trace id, kind, label, start, end, status, a `query` event's driver, SQL, rows and call site id | a request's timeline |
| `dumps` | dump: trace id, file, line, the bounded tree | a request's dumps |
| `logs` | log record: trace id where it has one, level, message, fields, source file and offset for a tailed line | the log view |
| `exceptions` | uncaught exception: trace id, class, message, file, line, group | a request's error |
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
