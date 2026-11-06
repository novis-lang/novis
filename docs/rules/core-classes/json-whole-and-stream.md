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
- `$source` is a `Core\IO\File`, which the member reads whole from its first byte and neither opens
  nor closes, or an `Iterable<bytes>` such as `Core\Request::bodyStream()` or a part's `content()`.
  Both are read through the one byte source every such member uses
  (`rule:core-api/a-member-takes-the-source-the-program-holds`).
- `Core\Json::encodeEach(Iterable<mixed> $items, { lines?: bool }): Iterable<bytes>` pulls one item at a
  time and gives the array's text in pieces of up to 64 KiB, brackets and commas included.
  `{ lines: true }` writes one value per line and no brackets.
- A request body is read with `Core\Json::decodeEach<T>(Core\Request::bodyStream(), '/items')`, so there
  is no `Core\Request::jsonEach`. A response writes `encodeEach`'s pieces to `Core\Response::stream` or
  `Core\IO::writeStream`, so there is no response member either.

**A walk is taken once.** The iterable reads forward, as `Core\Csv::rows`' walk does. A second `foreach`
goes on from the element after the last one given, so after a `break` it gives the rest, and once the
walk has ended it gives nothing.

**A failure halfway throws where it is found.** The elements already given to the program stay given,
and the `ParseError` names the line, the column and the byte offset in the whole document. A program
that must apply all or nothing wraps the walk in a transaction.

**Outside the pointer, nothing is built.** A value on the way to `$at`, or after the array ends, is
checked for syntax and depth and then skipped, and a skipped string is not kept. The document after
the array is read to its end, so trailing garbage still fails the walk. A skipped string value is read
without a copy: the walk peeks at a skipped value's first byte, and a `"` is read by the parser's
ignoring path, which keeps nothing. A skipped array or object is read by the walk's own skip again, so
its depth is still checked. A skipped object's **keys** do pass through the parser's scratch buffer,
whose capacity stays at the longest key until the walk ends: that is O(longest key), counted against
the request, and never O(document).

**JSON Lines is values separated by whitespace.** Each value is an element and blank lines are allowed.
Two values on one line are read as two elements, as every JSON Lines reader reads them, because the
parse cannot tell a newline it consumed after a number from one it did not.

**The reader is serde_json, driven inside a coroutine** that suspends after each element and whenever
it needs the next chunk. JSON keeps one parser, and a message names a position in the whole document.
`crates/nvs-stdlib/src/json.rs`'s module doc, § *A document read one element at a time*, holds the
probe's figures. The coroutine builds each element as `decode` builds a document and runs no compiled
code. A typed walk then builds `T` from that value on the task's own stack, through the tree-first path
`decodeAs` takes for a class that reads its own document, so a constructor never runs on a suspended
stack. A typed element is built twice, which is time spent per element in place of a second decoder
that runs compiled code across suspensions. `crates/nvs-stdlib/src/json/each.rs` owns the walk.

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
  Rewriting a skipped value inside the byte feed is the same framer, and is declined with it.
- **The parser's ignoring path for a whole skipped value.** It checks no depth and keeps one byte per
  open level, so it would trade the string's copy for a nesting one and drop `maxDepth`.
