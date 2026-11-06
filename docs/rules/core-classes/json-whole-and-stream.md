`Core\Json` is one subsystem in two shapes. `encode`, `decode` and `decodeAs<T>` take or give a whole
document, so its size is bounded by the request's memory. `decodeEach<T>` and `encodeEach` read and
write one element of an array, or one line of JSON Lines, at a time, so their memory is O(one element)
at any document size. Each element goes through exactly the decoder and encoder the whole document
does: the same types, the same checks, the same messages, the same taint and the same `secret`
refusal. A program picks the shape that fits how much of the document it needs at once.

**The two members, and no other door.**

- `Core\Json::decodeEach<T>($source, string $at = '', { lines?: bool, maxDepth?: int }): Iterable<T>`.
  `$at` is a JSON Pointer (RFC 6901) naming an array inside the document, and `''` is the document
  itself. Each element is decoded and checked exactly as `decodeAs<T>` checks a document, and `mixed`
  reads each element as `decode` would. `{ lines: true }` reads JSON Lines, one value per line, and `$at`
  must then be `''`.
- `$source` is a `Core\IO\File`, which the member reads forward and neither opens nor closes, as
  `Core\Csv::rows` does, or an `Iterable<bytes>` such as `Core\Request::bodyStream()` or a part's
  `content()`.
- `Core\Json::encodeEach(Iterable<mixed> $items, { lines?: bool }): Iterable<bytes>` pulls one item at a
  time and gives the array's text in pieces of up to 64 KiB, brackets and commas included.
  `{ lines: true }` writes one value per line and no brackets.
- A request body is read with `Core\Json::decodeEach<T>(Core\Request::bodyStream(), '/items')`, so there
  is no `Core\Request::jsonEach`. A response writes `encodeEach`'s pieces to `Core\Response::stream` or
  `Core\IO::writeStream`, so there is no response member either.

**A walk is taken once.** The iterable reads forward, and a second `foreach` gives nothing, as
`Core\Csv::rows` does.

**A failure halfway throws where it is found.** The elements already given to the program stay given,
and the `ParseError` names the line, the column and the byte offset in the whole document. A program
that must apply all or nothing wraps the walk in a transaction.

**Outside the pointer, nothing is built.** A value on the way to `$at`, or after the array ends, is
checked for syntax and depth and then skipped, and a skipped string is not kept. The document after
the array is read to its end, so trailing garbage still fails the walk.

**The reader is serde_json, driven inside a coroutine** that suspends after each element and whenever
it needs the next chunk. JSON keeps one parser, and a message names a position in the whole document.
`crates/nvs-stdlib/src/json.rs`'s module doc, § *A document read one element at a time*, holds the
probe's figures.

**Bounds.** Every element is built under the request's memory limit, and the walk runs under its CPU
and wall-time limits. A byte stream keeps the bounds its source documents, such as `bodyStream`'s. No
setting is added.

This is the second place in `Core` where two shapes of one subsystem coexist, beside
`rule:core-classes/xml-tree-and-stream`, and it is named for the same reason: the whole shape walks a
value in any order, and the stream holds O(one element) of a document larger than memory. Neither
shape can offer the other's property.

**Memory:** the whole shape is proportional to the document, attributed to the request and released
with it. A `decodeEach` walk holds one chunk of its source, the element being built and one coroutine
stack, resident only in the pages the parse touches. An `encodeEach` walk holds one piece of at most
64 KiB and the item being written.

**Declined:**
- **An event API** (start of object, key, value). Element by element covers the workloads, and is far
  simpler to use.
- **`Core\Request::jsonEach`.** It would be a second door to `decodeEach` over `bodyStream()`.
- **A whole shape only.** It cannot serve a document larger than the request's memory ceiling, which is
  a real workload.
- **A pull-reader crate.** It parses values with its own grammar and writes its own messages, so every
  message the whole decode gives would differ for the same document.
- **A framer that finds each element's bytes and hands them to the whole decoder.** It ran 1.5 times
  faster than the coroutine in the probe, but it is a hand-written parser of JSON's outer structure,
  which RFC 8259 being a dependency rules out, and its messages name a column inside the element.
