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

A part's bytes are readable exactly once, in order. An application that needs two passes buffers with `readAll` or writes the part down first, and now says which. Streaming-only was rejected because a small avatar to hash or a CSV about to be parsed wants bytes, and forcing a loop on them buys no safety once the bound moved to the call site.
