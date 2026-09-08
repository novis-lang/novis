---
summary: one uploaded part — what it declared about itself, and the three ways to spend its bytes
keywords: $_FILES, tmp_name, move_uploaded_file, UPLOAD_ERR_OK, upload name, client filename, Content-Disposition, Content-Type, multipart part, file_get_contents upload
---

A `Core\Request\Part` is one file part of a `multipart/form-data` body, handed to the loop body by the
`Core\Request::files()` walk. Three members say what it declared about itself — `name()`, the form field
it arrived under; `filename()`, the name the client claimed; and `contentType()`, which answers
`text/plain` where the part declared none, RFC 7578's own default rather than a repair of a missing value.
All three are `tainted string`: they are what a peer wrote back, and a client is free to send a field name
the form never declared.

**`filename()` is a claim about a file on someone else's machine, and never a path on this one.** As a
`tainted string` it reaches a path only through `Core\IO::within`, which is the same refusal every other
untrusted string meets — and it is the refusal that matters most here, since the one place an upload could
choose where it lands is the one place the qualifier stands in the way.

**There is no `size`.** Nothing honest can be said about a part's length before it has been consumed, and
inventing a number is the kind of repair this language refuses everywhere. A program that needs the count
gets it from the bytes it read.

**Three ways to spend a part, and each of them spends it.** `readAll({max?})` pulls the whole part into
one `tainted bytes` — the reading for an upload small enough to hold — bounded by `[limits] request_body`
(8M) when `max` is omitted, by that number when it is named, and refusing outright a `max` larger than the
request's own `[limits] memory` rather than quietly clamping it. `content()` walks the part a chunk at a
time, holding none of it, for an upload that must never be resident whole. `saveTo($path, {max?,
overwrite?})` writes it straight to disk one chunk at a time — the path almost every upload takes — under
the `fs.write` capability for that path; it is `Core\IO::writeStream` underneath, so the path must not
already exist unless `overwrite` says so, and a write that fails leaves no partial file behind.

**All of them are valid only while this part is the walk's current one.** A part kept past the iteration
that opened the next names bytes the parse has already drained, so it is refused rather than answered with
the current part's content.

```nvs skip
<?nvs
foreach (Core\Request::files() as Core\Request\Part $part) {
    int $bytes = 0;
    foreach ($part->content() as tainted bytes $chunk) {
        $bytes = $bytes + Core\Bytes::length($chunk);
    }
    echo $part->name(), " (", $part->filename(), ", ", $part->contentType(), "): ", $bytes, "\n";
}
```

The same part, held whole or sent to disk instead — one part is spent once, so these are two programs and
not two halves of one:

```nvs skip
<?nvs
foreach (Core\Request::files() as Core\Request\Part $part) {
    tainted bytes $whole = $part->readAll({max: 1 * 1024 * 1024});
    echo Core\Bytes::length($whole), "\n";
}
```

```nvs skip
<?nvs
foreach (Core\Request::files() as Core\Request\Part $part) {
    $part->saveTo(Core\IO::within("uploads", $part->filename()), {overwrite: true});
}
```

Every one of those needs a request in front of it; `Core\Request\Files` is where the walk that produces a
part is described, and what it does in a program answering no request.
