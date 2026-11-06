```nvs
foreach (Core\Request::files() as $part) {
    $csv = $part->readAll();                          // bounded by request_body
    $big = $part->readAll(max: 200 * 1024 * 1024);    // bounded by [limits] memory
    foreach ($part->content() as $chunk) { … }        // Iterable<bytes>, each tainted
    $part->saveTo($dest, max: 50 * 1024 * 1024);      // straight to disk
}
```

- **`readAll({max?}): tainted bytes`** — with no argument, bounded by `[limits] request_body` and refused past it. An explicit `max` is a count of bytes checked against the request tree's own `[limits] memory` instead. `request_body` governs what arrives unasked; `[limits] memory` governs what the application chooses to hold, which is what leaves a deliberate large buffer expressible while keeping the bare call safe by construction.
- **`content(): Iterable<bytes>`** — the part's chunks, each `tainted`, valid only while this part is the iterator's current one. A part the walk has moved past refuses rather than reading the current one.
- **`saveTo(string $path, {max?, overwrite?})`** — the shorthand for `rule:core-classes/io-write-stream`, to which it delegates under the same bag, and the path nearly every upload takes. Its path is a sink: a `tainted` `filename()` reaching it is a compile error, and `Core\IO::within` is the launderer.

A part's bytes are readable exactly once, in order, whichever of the three consumed them. The buffering-and-streaming split of `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it` is about the request body and does not repeat itself here. An application that needs two passes buffers with `readAll` or writes the part down first, and now says which.

**A part is held only by a prefix or random read, until the walk advances.** Two kinds of read take a part as a `Core\Bytes::Source` (`rule:core-api/a-member-takes-the-source-the-program-holds`) and consume nothing:

- **`peek(uint $max): tainted bytes`** and `Core\Mime::detect` read its start. The bytes read are held on the part. A second `peek` with a larger `max` reads further. A `max` over `[limits] memory` throws before a byte is read.
- **A random read** (`Core\Zip`, `Core\Image`, `Core\Pdf`) holds the whole part, under bare `readAll`'s `[limits] request_body` bound. Past the bound, the error names `saveTo`, a `Core\IO\File` and `readAll({ max })`.

Each of the three consumers gives the held bytes first and then the rest off the wire, so `Core\Mime::detect($part)` followed by `saveTo` writes the whole file. The hold is freed when the walk opens the next part. `Core\Request\BodyStream` behaves the same over a raw request body: its own `peek(uint $max)`, `Core\Mime::detect` and a random read hold what they read, and the `foreach` loop gives the held bytes first. A prefix read after a consumer has started, or on a part the walk has passed, throws `LogicError`, because the start it would give is gone. `readAll` buffering into the application's own variable still leaves nothing behind on the request for a second reader to find.

**Declined:**
- **Streaming only, with no way to read a part into memory.** A small avatar to hash, a CSV about to be parsed or a value for a database column wants bytes, and forcing a loop on them buys no safety once the bound sits at the call site.
- **A `max` required at every `readAll`.** The bare call is already bounded by `request_body`, the cap on every other byte parsed into memory, so the argument would buy visibility and no safety, at one argument per call site.
