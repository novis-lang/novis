# The performance pass — what it found

What goal `performance-pass` found, fixed and left for a decision. `bun nv scaling` is the tool, and
its module doc, [`tools/nv/cmd/scaling.ts`](../../tools/nv/cmd/scaling.ts), owns the ramp and the
bounds; [`benches/scaling/README.md`](../../benches/scaling/README.md) says what a ladder is.

## What we found

`bun nv scaling --iterations` over the whole bench tree: no bench's counts rise per operation as
its batch grows. Every bench it could judge is flat on all four counts.

## What got better

Nothing yet: no fix has landed.

## Decisions for you

None yet.

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

## What we checked and found fine

The 953 benches `bun nv scaling --iterations` judged flat, `.scale.nvs` and `.twin.nvs` siblings
included, at batches from 16 to at most 4096.
