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

Nothing yet: no fix has landed.

## Decisions for you

**Should a request head get a total deadline as well as its idle one?** `header_timeout` is an idle
wait: every byte that arrives restarts it (`crates/nvs-server/src/io.rs:309-321`), as
`rule:http-server/four-idle-waits-all-finite` decides. A client that sends its head one byte at a
time, just inside the wait, holds a connection for as long as it likes, and the HTTP layer parses its
buffer again on each read up to its own buffer limit. A total deadline for the head alone closes
that. It changes a rule and the behaviour a slow client sees, so it is yours to decide. Not
measured yet.

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

**A ladder whose allocation count grows faster than its work**:
[`benches/scaling/numbers/bigint-mul.nvs`](../../benches/scaling/numbers/bigint-mul.nvs) squares a
number of 64,000 to 512,000 bits. `num-bigint` uses Toom-3 at these sizes, which grows as n^1.47.
The tool reports allocations with slope 1.85 and bytes with 1.69, so it fails the ladder. The clock
follows the algorithm: a probe timing four multiplications, best of five runs, grew 60 times from
32,000 to 512,000 bits, which is slope 1.47. The allocations are the library's temporaries at each
level of the recursion, and the tool has no way to judge a count against the algorithm rather than
the size. The size cannot go higher: one `Core\BigInt` result is at most 1,048,576 bits.

## What we checked and found fine

The 953 benches `bun nv scaling --iterations` judged flat, `.scale.nvs` and `.twin.nvs` siblings
included, at batches from 16 to at most 4096.

The ladders `bun nv scaling` judged flat: `arrays/sort`, `compiler/functions`, `traffic/requests`,
`values/copy`, `strings/build`, `strings/split-join`, `regex/subject`, `json/roundtrip`,
`time/days`, `markup/xml-parse`, `formats/csv`, `formats/query`, `templates/rows`,
`database/rows`, `cache/keys`, `queue/jobs`, `scheduler/tasks`, `request/headers`,
`request/header-bytes`, `request/body`, `request/query` and `connections/open`.
