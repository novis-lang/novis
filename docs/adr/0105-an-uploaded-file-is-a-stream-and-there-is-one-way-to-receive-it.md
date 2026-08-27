# ADR 0105 — An uploaded file is a stream, and there is one way to receive it

- **Status:** Accepted
- **Date:** 2026-08-27
- **Scope:** how an uploaded file reaches an application and what bounds it — the single receiving
  accessor, what a part is, the three ways to consume one, where a stream reaches disk, and the two caps
  that replace one. Not in scope: the raw request body, which stays
  [0097](0097-development-server-and-proxied-origin.md) § 8; how a multipart message's *spelling* is read,
  including the part-count cap, which stays
  [0095](0095-ambiguous-input-is-refused-never-repaired.md); what `tainted` means and where it must be
  laundered ([0024](0024-taint-tracking-for-injection-sinks.md)); the changeability classes the two caps
  are instances of ([0005](0005-config-changeability.md)); and everything else about the server
  ([0097](0097-development-server-and-proxied-origin.md)).
- **Depends on:** [0097](0097-development-server-and-proxied-origin.md) — § 8 is the decision this one
  replaces, and the server whose body reader this is.
- **Amends:** [0097](0097-development-server-and-proxied-origin.md) § 8 — the buffered `files()` array is
  withdrawn, `request_body` is redefined as a cap on bytes parsed into memory rather than on the buffered
  path, `upload_total` joins it, and the *Consequences* line reading "the largest buffered upload equals a
  request's memory budget" is replaced by this ADR's disk-spend line.
  [0064](0064-configuration-file-format.md) § 2a — `[limits]`/`[limits.hard]` gain `upload_total` beside
  `request_body`, for the same reason that ADR already gives: an inbound body is per-request cost and
  belongs to the pair that governs per-request cost, not to a third one of its own.
  [0024](0024-taint-tracking-for-injection-sinks.md) — its M7 roster names the accessor `::files()` and a
  part's `filename`, `contentType` and every chunk it yields are what carry the taint, the eager `bytes`
  member having gone.
  [docs/spec/01-core-library.md](../spec/01-core-library.md) § 14 — `Core\IO` gains `writeStream`; § 15 —
  `Core\Request::files` returns `Iterable<Part>` and the `Part` shape replaces the `name`/`contentType`/
  `content` triple.
  [docs/plan/m7.md](../plan/m7.md) — M7 gains this ADR's surface and the second cap.

> **In short:** `Core\Request::files(): Iterable<Part>` yields uploaded files lazily, one part at a time,
> and is the **only** way to receive one. The buffered array [0097](0097-development-server-and-proxied-origin.md)
> § 8 specified is withdrawn, because choosing it correctly required knowing a size the application cannot
> observe until it has already paid for it. A part is consumed in one of three ways — `readAll` bounded by
> `request_body`, iterated as `Iterable<bytes>`, or written straight to disk by `saveTo`, which delegates
> to a new `Core\IO::writeStream`. Peak memory for an upload of any size is therefore one chunk per
> in-flight request. Two caps replace one: `request_body` (`"8M"`/`"64M"`) now means **bytes parsed into
> memory** and `upload_total` (`"256M"`/`"2G"`) bounds a **streamed multipart body**. There is still no
> temp file — no `tmp_name`, no temp directory, no upload/move race — because where a part lands is an
> application's explicit call under `fs.write`, never the runtime's default.

## Context

[0097](0097-development-server-and-proxied-origin.md) § 8 gave an application two ways to read an upload
and asked it to pick: a buffered `files()` returning each part's bytes eagerly, capped by
`[limits] request_body`, and `bodyStream()` for a body larger than the request's memory budget. Three
things are wrong with that, and only the third is about memory.

**The choice cannot be made correctly.** A per-part size is not knowable until the part has been consumed.
`Content-Length` describes the whole body, not the file inside it, and a client's claim about either is a
claim. So "use the buffered accessor when the file is small" is advice with no observation behind it: by
the time the application could check, the buffering has already happened. An API whose safe path is
selected from a fact the caller cannot see is a defect independently of what the unsafe path costs.

**The streaming path had no byte cap at all.** § 8 bounded the buffered path with `request_body` and left
the streaming one to `body_idle_timeout` "and whatever the consumer does with each chunk". That was
tolerable only while the buffered path existed as the bounded default; as the answer for large uploads it
means the recommended path is the unbounded one.

**And the buffered path cost twice what it looked like.** A `bytes` value in this runtime is a
`StrHeader { refcount, len, cap }` with its payload allocated inline and contiguous
([`crates/nvs-runtime/src/string.rs`](../../crates/nvs-runtime/src/string.rs)); there is no
slice-of-parent representation. A part's `content` therefore cannot alias the accumulated body buffer and
has to be copied out of it, so peak residency is the raw body *plus* the copied parts, live together until
the raw buffer drops. A permitted 2G upload is a 4G peak, per in-flight request, held for as long as the
client takes to send — and `max_in_flight` bounds request count, not bytes.

Underneath all three: `bodyStream()` is *raw* body bytes and is exclusive with `files()`, so there was no
streaming path for the one shape that actually needs streaming. A large browser upload is multipart,
always. The application's only alternatives were to buffer it or to implement RFC 7578 framing in Novis —
boundary detection across chunk edges, per-part headers, transfer encodings — which is precisely the
parser [0095](0095-ambiguous-input-is-refused-never-repaired.md) makes the runtime own, because a
parser differential is a security property and not an application's business.

## Decision

### 1. `Core\Request::files()` is a lazy iterator, and it is the only way to receive an uploaded file

`Core\Request::files(): Iterable<Part>` yields parts over
[0053](0053-iteration-and-generators.md)'s protocol as they arrive off the connection. It is single-pass,
and consuming it is exclusive with `body()` and `bodyStream()` on one request — the same exclusivity
[0097](0097-development-server-and-proxied-origin.md) § 8 already states, now stated once for one member
rather than between two.

There is no buffered sibling. The name is reused rather than retired: it is the obvious name for the
thing, nothing is built yet so there is no compatibility cost, and a `fileStream()` would imply a
`files()` that deliberately does not exist.

**Advancing the iterator past an unconsumed part drains it.** Skipping an upload the application does not
recognise is simply not touching it; the discarded bytes are still charged against § 5's `upload_total`.
The alternative — refusing to advance until the application calls `skip()` — was rejected in
*Alternatives*: it turns the ordinary "ignore what I don't know" loop into ceremony and prevents no
mistake anyone makes.

### 2. A part is a file part iff `Content-Disposition` carries `filename`

That is RFC 7578's own distinction and Novis does not invent a second one. Every other part is an ordinary
form field: **the server buffers it and `Core\Request::post()` works exactly as it does for a urlencoded
form.** This is what keeps ordinary form handling ordinary — a `<form>` with a title, a description and a
file is read the way it is written, and the application is not made to reconstruct `post()` out of part
order.

Those buffered fields are charged against `[limits] request_body`, not against a third directive: form
field text is bytes parsed into memory, which is exactly what § 5 makes that cap mean.

A `Part` carries `name` (the form field name), `filename` (the client's claimed name — `tainted`, and
never a path) and `contentType` (`tainted`). **It carries no `size`.** There is no honest value to put
there before the part has been consumed, and inventing one is the repair
[0095](0095-ambiguous-input-is-refused-never-repaired.md) exists to forbid.

### 3. Three ways to consume a part

```nvs
foreach (Core\Request::files() as $part) {
    $part->filename;                                  // tainted string — never a path
    $part->contentType;                               // tainted string

    $csv = $part->readAll();                          // bounded by request_body
    $big = $part->readAll(max: "200M");               // bounded by [limits] memory
    foreach ($part->content as $chunk) { … }          // Iterable<bytes>, each tainted
    $part->saveTo($dest, max: "50M");                 // straight to disk
}
```

- **`readAll({max?}): tainted bytes`** — with no argument, bounded by `[limits] request_body` and refused
  past it. An explicit `max` is checked against the request tree's own `[limits] memory` instead. The
  split is what makes the bare call safe by construction while leaving a deliberate 200M buffer
  expressible: `request_body` governs what arrives unasked, `[limits] memory` governs what the
  application chooses to hold.
- **`content: Iterable<bytes>`** — the part's chunks, each `tainted`, valid only while this part is the
  iterator's current one.
- **`saveTo(string $path, {max?, overwrite?})`** — the shorthand for § 4, and the path 99.9% of uploads
  take.

### 4. `Core\IO::writeStream` is where a stream reaches disk

`Core\IO::writeStream(string $path, Iterable<bytes> $src, {max?, overwrite?})` is the real member and
`saveTo` delegates to it — path first, because [0063](0063-core-api-conventions.md) R1 makes the subject
parameter 1 and `Core\IO::write` already reads that way. It is a `Core\IO` member and therefore already an
[0024](0024-taint-tracking-for-injection-sinks.md) **path sink** requiring `fs.write`, with no exemption
for arriving by way of an upload — a `tainted` filename reaching it is a compile error exactly as it is
everywhere else, and `Core\IO::within` is the launderer.

Two rules are its own. **`overwrite` defaults to false**, because a destination chosen from a client's
claimed filename is the case this member exists to serve. **A write that fails mid-stream removes the
partial file**, because a truncated file the application believes it wrote is a worse failure than an
error: the caller learns from the throw, not from a later reader.

It is general on purpose. `bodyStream()`, a decompressed archive, an `Core\Http` response body and an
upload part all reach disk through this one implementation, so the partial-write cleanup lives in one
place rather than in every call site that hand-wrote the loop.

### 5. Two caps, both rows in `[limits]`/`[limits.hard]`

| Directive | Default | Ceiling | Bounds |
|---|---|---|---|
| `request_body` | `"8M"` | `"64M"` | **Bytes parsed into memory** — `body()`, a JSON or urlencoded body, § 2's buffered form fields, a bare `readAll()`. |
| `upload_total` | `"256M"` | `"2G"` | **Total bytes of a streamed multipart body**, consumed parts and drained ones alike. |

Both are new rows in the `Runtime`-default-plus-`System`-ceiling pair
[0005](0005-config-changeability.md) established for `[limits]`, not a third instance of that pattern —
[0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)'s "second and last instance"
wording is about the pattern, whose second instance is `[mode] default`/`ceiling`, and adding a row to the
first instance leaves it true.

**`upload_total` is enforced by the server, on the wire.** A request whose `Content-Length` already
exceeds it is refused before dispatch, so the honest oversized client still never reaches application
code; a chunked body with no declared length is enforced as bytes arrive, and the iterator throws at the
part being read when the total is crossed. That the cap is enforced in the server rather than by each
consumer is what keeps the streaming path bounded now that it is the only path.

Splitting one cap in two follows from what the two now measure. Content parsed into memory deserves the
tighter number, because it is resident and it is O(in-flight); content streamed to disk deserves a
ceiling shaped like what uploads actually are, because a phone photo is 5–12M and an 8M refusal is advice
rather than protection. One number governing both would have to be the larger, which would license a 256M
JSON body — parsed wholly into memory — on the strength of an argument made about files.

[0095](0095-ambiguous-input-is-refused-never-repaired.md)'s multipart **part-count** cap applies
unchanged and is not restated here; it bounds bookkeeping, which no byte cap reaches.

### 6. There is still no temp file

Nothing here reintroduces what [0097](0097-development-server-and-proxied-origin.md) § 8 refused, and the
distinction is not a fine one. There is no `tmp_name` on a request, no temp directory to configure,
permission or defend against symlinks, no window between the runtime writing a file and the application
claiming it — so no upload/move race and no `is_uploaded_file` to bypass — and no orphan for the runtime
to clean up after a crash. A part that lands on disk lands there because the application named a path and
holds `fs.write`.

What that trades is stated rather than implied: **the resource this design can exhaust is disk, and the
application owns bounding it.** `writeStream`'s `max` is the per-call bound, `upload_total` is the
per-request one, and neither is a quota over a directory — an operator accepting uploads has to watch the
volume they land on. That is a cost Novis previously did not have, and it is bought with the memory cost it
previously did.

## Consequences

- **Peak memory for an upload of any size is one chunk per in-flight request**, plus whatever a call site
  explicitly asked to hold. The 2× copy of *Context* is gone with the accessor that caused it, and no
  configuration change can reach it.
- **Every application that accepts a file writes a loop**, where PHP wrote `$_FILES[…]['tmp_name']`. For
  the common case that loop is three lines and one of them is `saveTo`; the tax is real and it is the
  price of the choice in § 1 being unavailable to get wrong.
- **Disk replaces memory as the resource an upload can exhaust**, per § 6.
- **A part's bytes are readable exactly once, in order.** An application that needs two passes over an
  upload buffers it with `readAll` or writes it down first — and now says which.
- **`upload_total` is a second number an operator has to know about.** `nvs info --config` prints both,
  and § 5's table is the only place their meanings are defined.
- **A slow client still holds a connection**, bounded by `body_idle_timeout` as before. What it no longer
  holds is the body.

## Alternatives rejected

- **Keep the buffered `files()` alongside the stream.** The status quo. Rejected in § 1: the choice
  between them cannot be made from anything the application can observe, and the failure mode of guessing
  wrong is the one this ADR exists to remove.
- **Streaming only, with no in-memory consumption at all.** Rejected in § 3. A 20K avatar to hash, a CSV
  about to be parsed, a blob heading for a database column — all want bytes, and forcing an iteration loop
  on them buys no safety once the bound moved to the call site. `readAll`'s cap is what makes buffering a
  bounded request rather than an unbounded default.
- **Require `max` at every `readAll`.** Considered seriously: it puts the number where the reader is. But
  the bare call is not unbounded — it is `request_body`, the same cap that governs every other byte
  parsed into memory — so the required argument buys visibility rather than safety, at one argument per
  call site.
- **A third directive for buffered form fields.** Rejected in § 2: a knob whose right value nobody knows,
  overlapping `request_body` almost entirely.
- **One raised cap covering body and upload alike.** Rejected in § 5: it would have to be the larger of
  the two, and would then license a quarter-gigabyte JSON body parsed wholly into memory.
- **Refuse to advance past an unconsumed part.** Rejected in § 1: ceremony on the common loop, preventing
  a mistake nobody makes.
- **Give `Part` a `size` from `Content-Length` or a part header.** Rejected in § 2: neither describes the
  part, and a plausible wrong number is worse than no number —
  [0095](0095-ambiguous-input-is-refused-never-repaired.md)'s rule applied to a field instead of a name.
- **Runtime-managed temp files with the path hidden from the application.** The obvious middle road:
  spool every part to a runtime-owned file, hand the application a handle it cannot turn into a path. It
  removes `tmp_name`'s worst edge but keeps every operational one — a directory to configure and
  permission, cleanup after a crash, disk exhaustion under no cap — and buys nothing `saveTo` does not,
  since the application ends up naming a destination anyway.

## Verification

The fixture list, extending
[0097](0097-development-server-and-proxied-origin.md)'s own at M7:

- A multipart body larger than any single in-memory bound is received in full through `files()` at
  **bounded resident memory**, asserted against a high-water mark, not against wall-clock success.
- A form mixing text fields and files populates `post()` with every field and yields exactly the file
  parts, in either interleaving order.
- `readAll()` with no argument refuses a part exceeding `request_body`; `readAll(max: …)` accepts past it
  and is refused by `[limits] memory`; a route that raised its own caps first accepts what a default
  configuration refuses.
- A body whose `Content-Length` exceeds `upload_total` is refused **before any application code runs**,
  asserted with a counter proving no isolate was allocated — the same assertion
  [0097](0097-development-server-and-proxied-origin.md) already makes for `max_in_flight`.
- A chunked body with no `Content-Length` that crosses `upload_total` mid-stream throws at the part being
  read, and the connection is closed rather than left half-consumed.
- Skipping a part and advancing yields the next part correctly, and the skipped bytes are charged.
- `writeStream` refuses an existing destination by default; a write interrupted mid-stream leaves **no
  file behind**, asserted by killing the source iterator and then stat-ing the path.
- A `tainted` `filename` reaching `writeStream`'s path argument is a **compile** error, and the same value
  through `Core\IO::within` is accepted — the case
  [0024](0024-taint-tracking-for-injection-sinks.md)'s M7 line already requires, now with an upload as its
  source.
- Using `files()` and `body()`/`bodyStream()` on one request is refused.
