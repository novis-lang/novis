# The performance pass — what it found

What goal `performance-pass` found, fixed and left for a decision. `bun nv scaling` is the tool, and
its module doc, [`tools/nv/cmd/scaling.ts`](../../tools/nv/cmd/scaling.ts), owns the ramp and the
bounds; [`benches/scaling/README.md`](../../benches/scaling/README.md) says what a ladder is.

## What we found

`bun nv scaling --iterations` over the whole bench tree: no bench's counts rise per operation as
its batch grows. Every bench it could judge is flat on all four counts.

**`nvs lsp` grows close to quadratically with the size of the open document**
([`benches/scaling/lsp/classes.nvs`](../../benches/scaling/lsp/classes.nvs): clock slope 1.91, and
1.98 on a second run). The open, the edit, completion, hover and references each grow faster than
linear, and `nvs check` on the same document is linear, so the extra cost is in the server's own
analysis. Not fixed yet.

**Each task a request starts holds about 1 MB against the request's memory limit.**
[`benches/scaling/scheduler/tasks.nvs`](../../benches/scaling/scheduler/tasks.nvs) grows linearly,
so it passes. But `Core\Task::map` over 300 labels stops the request at the default ceiling of
256 MB: the release build reports 315,284,654 bytes held, which is 1.05 MB per task. So one request
can run about 250 tasks at once. The ladder stays at 128 tasks or fewer. Where the megabyte goes is
not checked yet.

**A POST body the program does not read can lose the response.** Checked over a raw socket
against `nvs serve` on a program that only echoes `ok` and never calls `Core\Request::body`. Up to
8 KB the connection stays open, and a request pipelined behind the POST is answered. From 16 KB the
POST is answered and the server then closes the connection, so the pipelined request gets nothing.
At 512 KB, 6 of 20 POSTs got a clean close and no response at all; at 64 KB and below, 20 of 20
were answered. So the server closes before its response is sent when much of the body is unread.
Where the close is decided is not checked yet.

**A request with 100 or more header lines gets `431 Request Header Fields Too Large`.** Checked
over a raw socket: 96 headers plus `Host` are answered, 100 plus `Host` get the 431 and a close. That
is a limit with a correct status, not a lost response. No limit is set under `crates/` (a grep for
`max_headers` finds none), so it is the HTTP layer's default. The `request/headers` ladder stays at
96 headers.

## What got better

**`Core\Request::post` and `::query` parse once per request.** Each call parsed the whole body or
query string again, so a handler that read each of P fields cost O(P²). The first call now holds
the parsed array on the request, and every later call is one lookup. The ladders
[`request/fields`](../../benches/scaling/request/fields.nvs) and
[`request/query-fields`](../../benches/scaling/request/query-fields.nvs) read every field of 64 to
4096 and are linear (clock slopes 1.01 and 1.13). The cost is one parsed array per request that
reads a form or a query, freed with the request. The before figure was not measured.

**A `Core\Log::write` below `[log] level` returns before it builds a record.** It converted the
fields, read the clock and took a coalescing slot first, so a filtered call cost O(fields), and
filtered records could evict the window of a record that is written. The ladder
[`traffic/debug-log`](../../benches/scaling/traffic/debug-log.nvs) makes n filtered calls with n
fields each: at 4000, the best of five alternating runs went from 497 ms to 41 ms, with the same
output. It spends nothing.

**`Core\Http\Stream::events()` and `::lines()` read each part of the reply once.** Each read of
8 KB that did not finish an event parsed the event again from its first line, so one event of n
bytes cost O(n²/8 KB). The parse now resumes where the last read stopped. The ladder
[`http-client/event-size`](../../benches/scaling/http-client/event-size.nvs) reads one event of 256
to 16384 lines: its bytes count grew with slope 1.92 before, and it is flat after. It spends
nothing.

**A cached regex answers what a fresh compile would.** A cache hit skipped the memory check a miss
makes, so a request with a low `[limits] memory` was refused on a cold core and served on a warm
one. Every use now checks it, and a program the compiler forced onto the backtracking engine is
never reused by a plain call. A full cache evicted all 256 programs at once. It now evicts the one
used least recently, so a hot pattern survives a stream of one-off patterns. The cost per call was
already bounded by the cache's size, so there is no ladder. Unit tests in the module cover it. It
spends one `usize` and one `u64` per cached program.

**A cache tier's eviction queue stays the size of what the tier holds.** A key that was forgotten
and written again took a second queue slot each time, so with `max_size` off a put-and-forget
cycle grew the queue with every request. An entry past its lifetime that nobody read again also
stayed forever. A write now sweeps the queue and the expired entries once the queue has doubled
since the last sweep, which is a constant per write on average. The queue holds at most twice the
live keys plus 64 slots. Each slot carries a number, so a stale slot never evicts the newer entry
under its key. The leak is in memory the clock does not see, so two unit tests in the module cover
it rather than a ladder. It spends one `u64` per entry and one per slot.

**A core's record of refused metric names stays at 256 keys.** Once `max_series` was reached, each
refused name added a key to a per-core map that was never trimmed, so a program that built metric
names from request data grew it with every request. The first 256 distinct names are still counted
by name, and refusals of any later name go into one counter. The growth is in memory the clock does
not see, so a unit test in the module covers it rather than a ladder. It spends at most 256 keys and
one `u64` per core.

**A throw no longer reads its message once for every frame it leaves.** Each frame that adds a
backtrace line also made an owned copy of the error's message and then discarded it. The copy is now
made only for a bare message that cannot become an object. The ladder
[`values/throw-depth`](../../benches/scaling/values/throw-depth.nvs) throws a message of 64 × n bytes
from n calls deep. It is flat on the counts before and after, because they do not see Rust's own
strings. On the clock, a 3 MB message thrown from 3000 calls deep took 48 ms before and 49 ms after,
the same as a four-call control. So the release build was already not paying for the copy. The
optimizer probably removed it, but that was not checked. The fix makes the copy absent in the code
rather than relying on the optimizer. A unit test in the module covers the message that cannot be
promoted. It spends nothing.

**Flattening a class collects each inherited name once.** Building a class's methods, constants,
fields and hooks checked every name against all the names collected so far, so a class of M members
cost O(M²) to compile. A name-keyed set now does that check. The ladder
[`compiler/members`](../../benches/scaling/compiler/members.nvs) declares one class of n methods, n
constants and n properties and a subclass of it. Its counts are flat before and after, because they
do not see the type checker's own work. On the clock, `nvs check` of 4000 members went from 234 ms
to 122 ms, and of 16000 from 1726 ms to 435 ms, best of three alternating runs. Each class still
walks its own ancestors rather than reusing its parent's layout: the layout is a full copy of what it
inherits, so reuse would save a constant factor. It spends one set of borrowed names per class while
it is built.

**Checking a function costs what its branches assign, not what is live at each one.** The checker
copied a function's whole set of definitely assigned locals for every `if` arm, loop body, `case`,
`catch`, `finally` and anonymous function, so a function of n statements checked in O(n²). Every branch now
runs on the one set, which keeps a log of what it gained: the branch is rewound off it and the join
puts back what every way out assigned (`crates/nvs-types/src/live.rs`). The ladder
[`compiler/branches`](../../benches/scaling/compiler/branches.nvs) prints one function of n locals,
each followed by an `if`. Its counts do not see the checker, but on the old binary it fails on
callgrind's instruction count, which grew with slope 1.95. On the clock, `nvs check` of 4096 went
from 301 ms to 43 ms and of 16384 from 4835 ms to 107 ms, best of three runs, before and after
alternated. It spends one more copy of each assigned name per function while it is checked.

**Checking a constructor costs what its branches assign, not what is assigned before each one.**
The check that every property is assigned on every path out of a constructor copied its set of
assigned properties for every branch, so a constructor of n properties, each assigned in an `if`,
checked in O(n²). It now walks every branch on one set and rewinds it the same way
(`crates/nvs-types/src/ctor_init.rs`). The ladder
[`compiler/constructor`](../../benches/scaling/compiler/constructor.nvs) prints one class of n
properties whose constructor assigns each in an `if` and its `else`. Its counts do not see the
checker. On the clock, `nvs check` of 16384 went from 12879 ms to 218 ms, best of four runs, before
and after alternated. It spends one more copy of each assigned property name per constructor while
it is checked.

**A file a `require` loads no longer copies the chain that loaded it.** Each newly loaded file took
a copy of its parent's require chain, and each `require` searched that chain for a cycle, so a chain
of n files cost O(n²) in time and in allocation. A file now carries only its depth: the walk keeps
one chain, cut back to that depth when the file is reached, and a set of the same paths answers the
cycle check in one probe (`crates/nvs-hir/src/requires.rs`). The ladder
[`compiler/requires`](../../benches/scaling/compiler/requires.nvs) prints n files, each requiring the
next and one shared file. It is flat on the old binary and the new one, and the clock could not tell
them apart even at 16384 files, about 3 s either way, because loading and parsing the files costs
far more than the copies did. It spends one more copy of each path on the chain while the program
is loaded.

**A `class<T>` conversion site looks up the classes that are a `T`.** Each `as class<T>` filtered
every class in the program and sorted the result, so C classes and C sites cost O(C²) in the
compiler's code generation. The lists are now built once per unit, one per supertype, from each
class's supertype set (`Classes::conforming` in `crates/nvs-codegen/src/lib.rs`). The ladder
[`compiler/class-sites`](../../benches/scaling/compiler/class-sites.nvs) prints n classes and one
function with n sites. It is flat, but its clock is `nvs check`'s, which stops before code
generation, so the fix was timed by hand: a cold `nvs run` of the printed program at 16384, old and
new binary in alternating rounds, took 12.5 to 14.1 s before and 8.2 to 11.2 s after. What remains
is the one long function, which grows about 5 times for 4 times the sites with only four classes.
It spends one label per edge of the class hierarchy for the life of the compiled unit.

**A request is matched only against the routes that share its fixed prefix.** `match_request` ran
`fill` on every row of the route table, so each request cost O(routes). The table now keeps a trie
keyed by each row's leading fixed segments, built once when the table is (`Prefix` in
`crates/nvs-runtime/src/routes.rs`). A request collects the rows along its own path and compares
them in load order, so which route wins does not change. The ladder
[`request/routes`](../../benches/scaling/request/routes.nvs) serves n routes and requests the last
one: on the old binary the clock grows with slope 1.06, and 1.04 on a second run; on the new one
the largest size is within 25% of the smallest. The ladder needed the new `size routes` of
`bun nv scaling`. A table whose rows all begin with a capture still compares every row. It spends
one trie node per distinct fixed prefix for the life of the table, and one list of candidate rows
per request.

**The `Core` signature table is built once per process.** Every check built the whole table from the
registry, interned its types and freed it again. The first check now builds it into a frozen base
(`crates/nvs-types/src/core_lib.rs`), and every check layers its own declarations and types over
that base. A check copies a native entry only when it must change it. The override pass also skips
a class with no supertype, which every native class is. Callgrind on the debug Linux binary, empty
program: `nvs check` went from 54.7 M instructions to 41.7 M, because the table is no longer freed
and the override pass no longer walks every native method. An `nvs lsp` session that opens an empty
file, edits it once and hovers went from 192.7 M to 48.6 M, because it builds the table once rather
than five times. The cost does not grow with input, so there is no ladder. It spends one copy of the
table for the life of the process, the same size as the copy each check built and freed.

**A scheduler turn reads only the tasks that need its attention.** The sweep at the end of each
turn read every parked task, a task ending searched its parent's whole list of children, and a
channel waiter leaving searched every waiter of its channel. With n tasks, each of these cost O(n),
and doing it once per task cost O(n²). The sweep now reads only the tasks cancelled since the last
turn. Children and waiters are kept in sorted collections keyed by id, so removing one is O(log n)
(`crates/nvs-host/src/scheduler.rs`, `crates/nvs-host/src/channel.rs`). The ladders
[`scheduler/fan-out`](../../benches/scaling/scheduler/fan-out.nvs),
[`scheduler/waiters`](../../benches/scaling/scheduler/waiters.nvs) and
[`scheduler/parked`](../../benches/scaling/scheduler/parked.nvs) are flat on the counts before and
after, because the counts do not see the scheduler. The repository's 256M memory cap stops the last
two at 128 parked tasks, so they were also timed by hand with no configuration file and no cap. The
release builds of the fix and its parent ran in three alternating rounds. `fan-out` at 32768
children took 578 ms before and 602 ms after, best of three. `parked` with 512 parked tasks and
512 sleeps took 62 ms before and 75 ms after, and with 2048 of each 122 ms before and 128 ms after.
Both builds grow linearly, so the clock on this machine does not see the
removed terms. A positive sleep wakes no sooner than the system timer, and one scan of 2048 tasks
costs far less than that wait. Callgrind was not run. It spends one tree node per child and per
waiter, and one list of task ids per turn.

**The editor's workspace index looks a name up in one probe.** `declaration`, `subtypes` and
`occurrences` each scanned every declaration or occurrence the index holds, and a CodeLens request
asks them once per declaration in the file, so a file of n classes cost O(n²) per request. The index
now keeps three maps, from a name to its declaration, from a supertype to its subtypes and from a
name to its uses, updated where a file enters and leaves the index (`crates/nvs-lsp/src/index.rs`).
The override walks keep the types they have seen in a set. Every `lsp` ladder now also asks for the
document's code lenses. The new ladder [`lsp/hierarchy`](../../benches/scaling/lsp/hierarchy.nvs)
prints n subclasses of one class that each override its method. The CodeLens request alone, best of
two alternating rounds, took 14, 36 and 178 ms at 800, 1600 and 3200 classes before, and 8, 18 and
39 ms after. Opening the file and the other requests still grew about three times per doubling,
and the next finding fixes that. It spends one path handle and one copy of the name per
declaration, per supertype edge and per occurrence, for as long as the file is indexed.

**The editor parses and links a file of n classes in O(n).** Two walks read the whole file again at
each class. The parser looked for a `///` run and a `/** … */` block above each declaration by
scanning every comment and space it had collected so far. A compile collects only comments, so only
the editor, which collects every space too, paid for it. The scan now stops at the first gap that
is not one line break of whitespace, because nothing further up can attach
(`crates/nvs-syntax/src/parser/mod.rs`). The hierarchy pass found where a `use` line would go for
each class by reading every statement above it. It now carries the last `namespace` and `use` line
as it walks (`SitesAbove` in `crates/nvs-hir/src/imports.rs`). That pass runs for `nvs check` too.
Callgrind on the debug Linux binary, one [`lsp/hierarchy`](../../benches/scaling/lsp/hierarchy.nvs)
session from open to code lenses: 1570 M and 4753 M instructions at 200 and 400 classes before,
764 M, 1492 M and 3007 M at 200, 400 and 800 after. `bun nv scaling lsp/` is flat. It spends
nothing.

## Decisions for you

**Should a request head get a total deadline as well as its idle one?** `header_timeout` is an idle
wait: every byte that arrives restarts it (`crates/nvs-server/src/io.rs:309-321`), as
`rule:http-server/four-idle-waits-all-finite` decides. A client that sends its head one byte at a
time, just inside the wait, holds a connection for as long as it likes, and the HTTP layer parses its
buffer again on each read up to its own buffer limit. A total deadline for the head alone closes
that. It changes a rule and the behaviour a slow client sees, so it is yours to decide. Not
measured yet.

**Should `Markup + Markup` append into the left operand when nothing else holds it?**
`nvs_core_html_markup_concat` (`crates/nvs-stdlib/src/html.rs:817-830`) copies both sides into a new
string, so `$page = $page + html`<tr>…</tr>`` over n rows copies the page so far on every row. A probe
of that loop, best of three on a release build with the 9 ms start-up floor taken off, ran 8000 rows
in 44 ms and 32000 rows in 856 ms: 19 times the time for 4 times the rows, which is quadratic. String
`.` is linear in the same loop because `InstKind::Concat` hands the first operand's reference to
`nvs_str_concat_n`, which writes in place at a refcount of one. The `Markup` fix needs the same two
pieces for a `CoreCall`, whose arguments are borrowed today: a lowering that hands over the left
operand's reference only where its holder is re-pointed at the result, and a helper that appends into
the carrier's text slot when both the object and its string have one reference. Both are new `unsafe`
code, so it is recorded rather than built. `Core\Html::join` is the linear form now. The gain is the
whole quadratic term, and it spends no memory beyond the doubling a string `.` already spends.

## Benches to look at

`bun nv scaling --iterations` could not judge these. Each still has its own record in the member
ledger; only the growth check over batches skips it.

**One run is the whole bench (`iterations 1`)**, so there is no batch to grow. Each reads one
request, one upload or one stream, and its growth belongs to a `serve` ladder over requests served:
`core/IO/stdin`, `core/Request/bodyStream`, `core/Request/files`, `core/Request-Part/content`,
`contentType`, `filename`, `name`, `readAll`, `saveTo`, `core/Socket/upgrade`, `core/Sse/upgrade`.

**The closing line does not pass `iterations` as one literal**, so the tool has no number to
rewrite. Most are request handlers or whole-program benches whose closing line builds a response or
a report:

- `core/Response/`: `addCookie`, `bytes`, `html`, `json`, `sendFile`, `setHeader`, `setStatus`,
  `slotted`, `stream`, `text`; `core/Response-Stream/write`
- `core/Test-Response/`: `body`, `cookies`, `header`, `headers`, `json`, `jsonAs`, `status`;
  `core/Test/request`
- `core/Script-ExitReport/`: `error`, `memoryPeak`, `reason`, `status`
- `core/Html/later`
- `lang/expressions/`: `assignment`, `is-new-clone-throw-print-exit-isset-empty`, `match`
- `lang/programs/`: `a-program-is-a-file-of-top-level-statements`, `code-mode-and-html-mode`,
  `comments`, `doc-comments`, `names-and-casing`
- `lang/statements/`: `break-and-continue`, `catch-as-an-expression`, `foreach`

**`requires: unix`**, skipped on a Windows host: `core/Net/connectLocal`, `core/Net/listenLocal`.
They run on the WSL leg.

**Unclear after callgrind**: `core/IO-File/tell`. Statements grow with slope 1.00 and callgrind's
instructions with 1.01, but the increments did not agree by the ceiling of 4096, so the tool does
not judge it.

**A ladder unclear because its algorithm grows in steps**:
[`benches/scaling/numbers/bigint-mul.nvs`](../../benches/scaling/numbers/bigint-mul.nvs). `num-bigint`
picks schoolbook, Karatsuba or Toom-3 by length and recurses a whole level deeper past each threshold,
so one size's allocation count is not monotonic: squaring 384,000 bits allocated 2,999 times and
256,000 bits 3,095 times. Squaring one size per step, the ladder read allocations at slope 1.85 and
failed. It now squares sixteen sizes from n/2 to 31n/32 per step, which reads allocations 1.31,
bytes 1.40 and callgrind's instructions 1.29, all under `karatsuba`'s 1.7 and Toom-3's own 1.47. The
local slopes still differ by more than the agreement test allows, so the tool reports it unclear and
does not judge it. The size cannot go higher: one `Core\BigInt` result is at most 1,048,576 bits.

## What we checked and found fine

The 953 benches `bun nv scaling --iterations` judged flat, `.scale.nvs` and `.twin.nvs` siblings
included, at batches from 16 to at most 4096.

The ladders `bun nv scaling` judged flat: `arrays/sort`, `compiler/functions`, `traffic/requests`,
`values/copy`, `strings/build`, `strings/split-join`, `regex/subject`, `json/roundtrip`,
`time/days`, `markup/xml-parse`, `formats/csv`, `formats/query`, `templates/rows`,
`database/rows`, `cache/keys`, `queue/jobs`, `scheduler/tasks`, `request/headers`,
`request/header-bytes`, `request/body`, `request/query` and `connections/open`.

`Core\Request::cookie` and `::headers` walk every header line on each call. The server answers
`431` from 100 header lines, so each walk is bounded by a constant and is not a growth defect.
