# The performance pass — the code review

Stage 4 of goal `performance-pass`: the code of every area in
[`tools/nv/cmd/scaling.ts`](../../tools/nv/cmd/scaling.ts)'s `AREAS`, read for the shapes the goal
lists. A ladder only finds what somebody made large, and this file is what reading the code found.
One read-only reviewer read each area. Each finding that grows is a gap record under
[`data/gaps/`](../../data/gaps/) owned by `performance-pass`, and the record is its home: the cause at
`file:line`, the ladder that would show it and the growth the fix should reach. A record says *from
the Stage 4 review, not re-run* where the site was not read again by the session that wrote it.
`bun nv scaling --areas --reviewed` checks that every area has a section here.

A finding that is bounded by a small fixed ceiling, or that costs a constant factor, is named under
*Bounded* and has no record, under the goal's no-micro-optimizations decision.

## compiler

**Read:** the parser's statement and guard code, the `require` walk, the class layout and locals
passes of the checker, IR lowering of classes, and codegen's class conversion.

**Found:** a long `elseif` chain is not depth-guarded and overflows the stack at 300 arms in the
debug build, and the same chain written `else if` is refused with `E0108` at 90
([`an-elseif-chain-is-not-depth-guarded`](../../data/gaps/nvs-syntax/an-elseif-chain-is-not-depth-guarded.json),
checked with a probe). Quadratic in program size:
[`a-require-chain-is-cloned-per-file`](../../data/gaps/nvs-hir/a-require-chain-is-cloned-per-file.json),
[`the-live-set-is-cloned-per-branch`](../../data/gaps/nvs-types/the-live-set-is-cloned-per-branch.json),
[`flattening-a-class-dedupes-by-linear-scan`](../../data/gaps/nvs-types/flattening-a-class-dedupes-by-linear-scan.json)
and [`conforming-to-scans-every-class-per-site`](../../data/gaps/nvs-codegen/conforming-to-scans-every-class-per-site.json).

**Bounded:** `ClassLayout::slot_of` is a linear search per property default
(`crates/nvs-ir/src/lower/mod.rs:316`), O(F²) in one class's fields. The duplicate-key check of a
shape literal scans linearly (`crates/nvs-types/src/expr/literals.rs:1019`), O(K²) in one literal.
Not checked: recursion depth of the checker and lowering on expressions that pass the parser's limit,
and whether `line_col` counts from the start of a file.

## values

**Read:** freeing, arrays, objects and property lookup, throwables and backtraces, closures, debug
records, and the per-request tables of open handles.

**Found:** a throw copies its message once per frame it unwinds
([`a-throw-copies-its-message-per-frame`](../../data/gaps/nvs-runtime/a-throw-copies-its-message-per-frame.json)).

**Fine:** freeing a long chain is one worklist loop, not recursion (`crates/nvs-runtime/src/release.rs:58`).
A backtrace push moves the array out of its slot, so copy-on-write never copies it. Array unset
compacts once holes reach half (`array.rs:744`). Property lookup tries the slot hint first and scans
only the class's declared fields. The debug record's cycle stack is bounded by its depth cap.
`open_files` (`ctx/held.rs:353`) keeps one slot per open for the request's life, as its doc says;
that is memory per request in the number of opens, freed with the request.

## arrays

**Read:** `Core\Arr` sort, unique, diff, intersect, contains, slice, splice, keys, merge and the
overlay family, and the array cursor.

**Found:** values that differ only below depth 4 hash alike, so `unique`, `diff` and `intersect`
over them are quadratic
([`identity-hash-stops-at-depth-four`](../../data/gaps/nvs-runtime/identity-hash-stops-at-depth-four.json)).

**Fine:** sort is a bottom-up merge sort over a permutation, O(n log n). `unique` without a
comparator hashes. `replaceRange` is three linear walks with no shifting. The overlay family uses an
explicit stack. `diff` and `intersect` with a comparator are O(n·m) closure calls by design: a user
comparator cannot be hashed. `contains` is O(n) per call, which is what it is. Not checked:
`crate::sort::natural`, and whether hole compaction holds for every cursor walk
(`crates/nvs-runtime/src/array.rs:777`).

## strings

**Read:** `Core\Str` indexing, slicing, search, replace, split, join, case and the grapheme
counting under them, and string concatenation.

**Found:** an index, a slice start or a search `from` walks the subject from its start on every call
([`a-string-index-walks-the-whole-subject`](../../data/gaps/nvs-stdlib/a-string-index-walks-the-whole-subject.json)).

**Bounded:** `replaceAll` tries every pair at every position, O(n·p), which its doc names
(`crates/nvs-stdlib/src/str.rs:2802`). A `debug_assert!` checks the whole joined buffer on each
concat (`crates/nvs-runtime/src/string.rs:1293`): quadratic in a debug build only.

**Fine:** concat grows in place and reuses the left operand. `replace`, `contains` and the find
family use KMP. `split`, `lines`, `chunk`, `graphemes` and `reverse` are one pass. `length` is
cached.

## regex

**Read:** `Core\Regex` compile and its cache, `match`, `matchAll`, `replace`, `split`, and the cursor
they share.

**Found:** a cache hit skips the memory check a miss makes, the cache key leaves out the engine, and
a full cache empties itself whole
([`a-regex-cache-hit-skips-the-memory-check`](../../data/gaps/nvs-stdlib/a-regex-cache-hit-skips-the-memory-check.json)).
`match` with `from` walks the subject from its start, under the strings record.

**Fine:** `matchAll`, `replaceWith` and the match builder share one forward cursor per call.
`replace` grows by doubling. A negative `split` limit makes two linear passes.

## json

**Read:** encode, decode, `decodeAs` and its issue paths, and the depth bounds of each.

**Found:** `decodeAs` builds a path for every list element and an issue for every bad one
([`decode-positions-reports-every-bad-element`](../../data/gaps/nvs-stdlib/decode-positions-reports-every-bad-element.json)).

**Bounded:** encode's cycle check scans the ancestor stack per container, and a field prefix is
copied once per nested class level. Both are bounded by `DEPTH_CEILING` (1024).

**Fine:** decode recursion is bounded by `max_depth` and runs on the spare stack past 64 bytes.
Encode is an explicit heap stack. Not checked: the `serialize` codec in `nvs-runtime`.

## markup

**Read:** the XML reader and tree builder, namespace scopes and the writer, and the HTML tree sink,
sanitizer and serializer.

**Found:** nothing that grows without bound.

**Bounded:** the sanitizer moves an unwrapped element's children into its parent once per unwrapped
level (`crates/nvs-stdlib/src/html.rs:1896`), capped at 512 open elements. Namespace lookup walks the
open scopes per read (`xml.rs:2367-2389`), capped at depth 1024. `detach` and `insert` shift a
sibling list (`html.rs:1077`, `:1087`); possible on misnested formatting tags, not shown.

**Fine:** XML refuses nesting past 1024 and HTML caps its stack at 512. Parse and write are iterative.
The XML attribute duplicate check switches to a hash set at 16 attributes. The writer joins its
pieces once.

## formats

**Read:** form, multipart, query and cookie parsing on the request, `Core\Uri`, multipart reading,
and CSV.

**Found:** `post`, `query` and `Core\Uri::queryParameter` parse the whole input on every call
([`request-post-and-query-reparse-per-call`](../../data/gaps/nvs-stdlib/request-post-and-query-reparse-per-call.json)).

**Bounded:** the multipart reader looks for a part's header end from the same start after every chunk
(`crates/nvs-stdlib/src/multipart.rs:439-450`), quadratic in a header block of at most 16 KiB.

**Fine:** CSV is one pass with a reused field buffer. The multipart body scan, bracket-path insert
and `remove_dot_segments` are linear.

## templates

**Read:** HTML and text escaping, value dumps, markup literal lowering, `echo`, and `Markup`
composition.

**Found:** `Markup + Markup` copies both sides, so a page built row by row is quadratic
([`markup-concat-copies-the-whole-left-side`](../../data/gaps/nvs-stdlib/markup-concat-copies-the-whole-left-side.json)).

**Fine:** a markup literal is lowered once at compile time into one concat and one escape per hole.
Escaping reads its input at most three times. Dump recursion is bounded by the model's depth cap.

## numbers

**Read:** `Core\BigInt` parse, format, arithmetic and conversions, `Core\Decimal`, and `Core\Math`'s
base conversions.

**Found:** nothing steeper than the algorithm.

**Bounded:** `toString`, `format` and `parse` use the library's radix conversion, which is O(n²) in
digits by what is known of it, not by reading its source; `MAX_BITS` (2²⁰) caps the input. The
`toInt` error message formats the whole value (`crates/nvs-stdlib/src/bigint.rs:1157`), which is that
conversion on an error path. Each operand is copied out of its slot per call (`bigint.rs:795-813`), a
constant factor.

**Fine:** multiply is Karatsuba or Toom-3, `pow` squares, `pow` and `shl` refuse past `MAX_BITS`
before working. `Decimal` is a fixed 96-bit value, so its digit count cannot grow.

## time

**Read:** `Core\DateTime` and `Core\Zone` members and their zone resolution.

**Found:** nothing. There are no range or recurrence generators to grow. The only loops walk fixed
field lists. Every member resolves its zone by name (`crates/nvs-stdlib/src/time.rs:3423`); the
library's own cache under that is not checked.

## database

**Read:** the placeholder rewrite and statement cache, binding, row hydration and execution in every
driver, the SQLite lock, and the pool lookup per request.

**Found:** nothing that grows with traffic or data.

**Bounded:** each named placeholder is found by a linear search over the arguments
(`crates/nvs-db/src/sql.rs:572`), O(p²) in one statement. The statement cache is a list in
most-recent order with a linear lookup (`sql.rs:399-406`), O(capacity) per execution, default 16.
`executeMany` rewrites the SQL for every set (`crates/nvs-stdlib/src/db/bind.rs:680`). A column
name is allocated per row (`db/execute.rs:490`). Each is a constant factor or bounded by a
statement's size.

**Fine:** the SQLite lock is per connection and taken inside the blocking pool. Row fields are read
by hashed key. Not checked: whether pools keyed by settings that carry credentials are ever trimmed
(`db/open.rs:770-789`), and the TDS driver.

## queue

**Read:** the jobs schema and indexes, the claim in every dialect, the idle poll, `purge`, `stats`
and the wake bell.

**Found:** a claim sorts the due backlog, and an idle poll reads the whole table
([`the-queue-claim-sorts-every-due-row`](../../data/gaps/nvs-stdlib/the-queue-claim-sorts-every-due-row.json)).
Finished jobs stay until purged, and `purge` and `stats` scan them
([`finished-jobs-are-kept-until-purged`](../../data/gaps/nvs-stdlib/finished-jobs-are-kept-until-purged.json)).

**Fine:** the bell's waiters are one per idle worker. The dedupe lookup uses its unique index.

## cache

**Read:** the request, local and process tiers, eviction, the fill lock, sessions, and
`Core\Storage::list`.

**Found:** a forgotten key leaves its eviction slot, so a put, forget and put cycle grows the tier
without bound
([`a-forgotten-cache-key-leaves-its-order-slot`](../../data/gaps/nvs-stdlib/a-forgotten-cache-key-leaves-its-order-slot.json)).

**Bounded:** a session `get` decodes the whole record and a `set` encodes it again
(`crates/nvs-stdlib/src/session.rs:1004-1080`), O(session size) per key, which its doc names as an
accepted trade. `Core\Storage::list` reads and sorts the whole directory per call
(`crates/nvs-stdlib/src/storage.rs:560-579`). The fill lock `FILLS` is one process-wide mutex, held
only around the map. Not checked: whether a session is written to the store for a request that sent
no session cookie (`session.rs:825-831`), and the Redis tier past a grep.

## scheduler

**Read:** the scheduler's run loop and task tree, timers, the stack pool, host channels, and
`Core\Channel`, `Core\Topic` and the bus.

**Found:** a channel send scans every waiter on the core
([`a-channel-send-scans-every-waiter-on-the-core`](../../data/gaps/nvs-stdlib/a-channel-send-scans-every-waiter-on-the-core.json)).
A scheduler turn scans all parked tasks, and a finished child scans its siblings
([`a-scheduler-turn-scans-every-parked-task`](../../data/gaps/nvs-host/a-scheduler-turn-scans-every-parked-task.json)).
The 1.05 MB per task that *What we found* reports matches the 1 MiB stack every task reserves
([`a-task-is-charged-a-whole-stack`](../../data/gaps/nvs-host/a-task-is-charged-a-whole-stack.json)).

**Fine:** timers are a `BTreeSet` with a map beside it. The bus publishes in O(cores).

## request

**Read:** the route table and its dispatch, mounts, CORS, header and cookie reads, the policy
snapshot, and the CSRF key.

**Found:** a request is matched against every route in turn
([`route-matching-tries-every-route`](../../data/gaps/nvs-runtime/route-matching-tries-every-route.json)),
and form and query reads parse again per call, under the formats record.

**Bounded:** `[http] csrf_key` is base64-decoded on every request (`crates/nvs-cli/src/serve.rs:1052`),
a constant per request. `Serving::policy` takes a read lock on one process-wide `RwLock` per request
(`crates/nvs-server/src/serve.rs:809`), which contends across cores rather than growing with traffic.
Header and cookie reads are quadratic up to the 100-header limit.

**Fine:** the route table is built once per compile, and the mount table only on a reload.

## connections

**Read:** the connection deadlines and phases, admission, the WebSocket wrapper and its bounds, and
the OTLP hand-over.

**Found:** a WebSocket whose peer stops reading keeps every frame sent to it
([`a-websocket-write-buffer-has-no-ceiling`](../../data/gaps/nvs-server/a-websocket-write-buffer-has-no-ceiling.json)).

**A rule, not a defect:** `header_timeout` is an idle wait, re-armed on every byte
(`crates/nvs-server/src/io.rs:309-321`, `:421`), as `rule:http-server/four-idle-waits-all-finite`
says. A client that sends a head one byte per interval holds the connection, and the HTTP layer
parses its buffer again on each read, up to its own buffer limit. Whether the head gets a total
deadline as well is under *Decisions for you* in [performance-pass.md](performance-pass.md).

**Bounded:** `hand_over` takes one process-wide mutex per sampled request, and its queue drops the
oldest past its ceiling. The clock is read once per read or write that moved bytes.

**Fine:** the drain generations, the WebSocket open count and admission are atomics or one slot per
connection.

## traffic

**Read:** every process-wide map, log, metric, rate-limit and cache table in the runtime, the
standard library and the server.

**Found:** the cache order queue, under the cache record. A log call below the level still builds
its record
([`a-filtered-log-call-builds-its-record`](../../data/gaps/nvs-stdlib/a-filtered-log-call-builds-its-record.json)).
Refused metric names grow a map without bound
([`metric-refusals-grow-with-distinct-names`](../../data/gaps/nvs-runtime/metric-refusals-grow-with-distinct-names.json)).

**Fine:** the OTLP queue, the fill table, the per-core metric registry and the process-tier shards
are each bounded. The rate limiter's state lives in the capped local tier.

## http-client

**Read:** the transport, the connection pool, redirects, body collection and the event-stream
reader.

**Found:** the event-stream reader parses its buffer from the start after every partial read
([`an-event-stream-reparses-its-buffer-per-read`](../../data/gaps/nvs-stdlib/an-event-stream-reparses-its-buffer-per-read.json)).

**Bounded:** each resolved address formats its pool key and hashes the whole trust anchor PEM again
(`crates/nvs-stdlib/src/http/transport.rs:1464-1467`). An expired idle connection is closed only by
a later call on the same core (`http/pool.rs:83`), within the pool's cap of 16.

**Fine:** the body is appended with amortized growth and capped. A redirect replaces one URL per hop,
up to `max_redirects`.

## lsp

**Read:** the workspace index, the server's edit and publish path, document overlays and positions.

**Found:** the index copies an entry's read list into every file it loads
([`the-index-copies-an-entrys-reads-into-every-file`](../../data/gaps/nvs-lsp/the-index-copies-an-entrys-reads-into-every-file.json)),
and dimming unused private members scans every occurrence once per declaration
([`unused-private-scans-every-occurrence`](../../data/gaps/nvs-lsp/unused-private-scans-every-occurrence.json)).
Both are candidate causes of `lsp/classes.nvs`'s slope near 2.

**Bounded:** each edit analyses again every entry whose graph read the changed file
(`crates/nvs-lsp/src/index.rs:344`, `server.rs:497`), linear per edit. `overlay` copies the text of
every open buffer into a new source map per call (`document.rs:229`, `:324-327`).

**Fine:** a position is a line-start table plus a scan from the line start, not from the file start.

## fmt

**Read:** indent placement, the printer, brace and space placement, and the syntax index under them.

**Found:** every lookup into the syntax index scans all nodes, once per line and per candidate byte
([`a-syntax-index-lookup-scans-every-node`](../../data/gaps/nvs-syntax/a-syntax-index-lookup-scans-every-node.json)),
and a line's indent recurses once per enclosing body
([`opening-of-recurses-per-enclosing-body`](../../data/gaps/nvs-fmt/opening-of-recurses-per-enclosing-body.json)).
These are the likely cause of `fmt/classes.nvs` growing.

**Fine:** the output string is appended to. Sorts run outside loops.

## app

**Read:** the `app` ladders under [`benches/scaling/app/`](../../benches/scaling/app/), route
dispatch, sessions, validation, the cache tiers and the inbound header list.

**Found:** what the shop program meets together is recorded under each feature's own area: route
dispatch under request, the session's whole-record access under cache, the order queue under cache.
No cost multiplies only where features meet. There is no middleware chain in the runtime or the
standard library to rebuild per request, and validation parses no rules per request.

Not checked: the outbound response header path, `csrf.rs`, and how far reflection runs per request.

## floor

What the smallest input costs before a program's own work starts: the file `<?nvs` and nothing
else, under a configuration of one `[[app]]` block. Counts are `--count`'s; instruction shares are
callgrind on the debug Linux binary; clocks are the best of 30 runs of the release binary on
Windows, beside `nvs --version` at 5.7 ms, which is the process start alone.

**Run:** `compile: tokens=2 nodes=1 names=0 exprs=0 ir=75` and `count: statements=1 calls=0
allocations=12 bytes=3617`. Most of the 75 IR instructions are the constructors of the four `Core`
throwables every program lowers (`nvs run --dump-ir`). The clock is 10.6 ms, 4.9 ms above the process start. A cold run
spends 44% of its instructions in the front end, 23% in the JIT and 11% in the object build the
artifact cache publishes, which `crates/nvs-cli/src/cache.rs:1552` pays on a cold key only and
on purpose. 11% is the table of helper addresses (`nvs_stdlib::symbols`).

**Compile:** `nvs check` is 8.3 ms, 2.6 ms above the process start. 54% of its instructions build
the `Core` signature table from the registry and another 8% drop it again
([`core-signatures-are-built-again-for-every-check`](../../data/gaps/nvs-types/core-signatures-are-built-again-for-every-check.json)).
The configuration is read once per process and grows linearly with its `[[app]]` blocks, about
35 µs each (800 blocks: 34.7 ms); this repository's own `nvs.toml` adds 15 ms, and a deployed
program's configuration of a few blocks adds about one.

**One request:** an empty request over a keep-alive connection is 47 µs, measured from a Bun
client that answers its own empty server in 43 µs. The client and the loopback are most of it, and
`nvs serve` prints no counts to divide it further.

**Editor:** an `nvs lsp` session that opens an empty file, edits it once and hovers is 13.1 ms,
7.4 ms above the process start. It runs the front end five times over two versions of the
document, and the `Core` signature table is 63% of its instructions
([`an-edit-analyses-the-document-twice`](../../data/gaps/nvs-lsp/an-edit-analyses-the-document-twice.json)).

Not checked: the share of the signature table in a check of a large program.
