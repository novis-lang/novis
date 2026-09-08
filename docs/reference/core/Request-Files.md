---
summary: the uploads a request carries, as a walk over its parts — `$_FILES` and `move_uploaded_file` replaced by a stream that touches no temporary directory
keywords: $_FILES, move_uploaded_file, is_uploaded_file, upload_tmp_dir, file upload, multipart/form-data, UPLOAD_ERR_OK, tmp_name, large upload, streaming upload
---

`Core\Request\Files` is what `Core\Request::files()` answers with: an `Iterable<Core\Request\Part>` that a
`foreach` walks once, yielding each uploaded part as it comes off the wire. It has no members — the walk
is everything it does — and it is the only way a program receives an upload. There is no temporary file
and no directory the host chose: nothing lands on disk until the program names a path it holds the
`fs.write` capability for, so an upload nobody saves has cost the machine nothing but the bytes that
crossed the socket. `$_FILES`, `tmp_name`, `is_uploaded_file` and `move_uploaded_file` all have no
replacement here because none of them has anything left to describe.

**The parse runs as the loop does.** The next part does not exist when the walk is named, so `files()`
holds one part at a time and nothing accumulates. A part is valid only while it is the walk's current one:
keeping one past the iteration that opened the next is holding a name for bytes the parse has already
drained, and it is refused rather than answered with the wrong part's content. Advancing past a part the
loop body never read simply drains it — skipping an upload the application does not recognise costs a walk
over bytes the door already charged for, and no memory at all.

**A part is a file part iff its `Content-Disposition` carries a `filename`**, which is RFC 7578's own
distinction and not a second one invented here. Every other part is an ordinary form field: the walk
buffers those as it passes them, and `Core\Request::post()` reads them back afterwards. That is why
`post()` on a `multipart/form-data` request is called **after** this walk and never before — the fields
are behind the uploads on the wire, and reading them first would mean draining the uploads to reach them.

The walk is empty where the request declared no `multipart/form-data` body, which is what a request
carrying no upload is. It throws `LogicError` where the program is not answering a request, or where
`body()`, `bodyStream()` or `post()` already read this body; `ParseError` where the request declared a
multipart body and then did not say how to read one — no `boundary`, two of them, or one outside the
grammar — since an ambiguous body is refused rather than guessed at; and `IOError` where the connection
failed under the body or the peer stopped short of the length it declared.

**What bounds it is the server, not this walk.** `[limits] request_body` bounds what is parsed into
memory and has nothing to say about a streamed body; the server carries at most 256M of one request body,
declared or chunked, refusing an oversize before the program runs and stopping a chunked one at the part
being read. That number is a constant today — the directive that makes it settable per deployment is not
on disk yet.

```nvs skip
<?nvs
foreach (Core\Request::files() as Core\Request\Part $part) {
    string $path = Core\IO::within("uploads", $part->filename());
    $part->saveTo($path);
    echo $part->name(), " saved\n";
}
```

`filename()` is `tainted`, so `Core\IO::within` above is not decoration: it is the one way a name the
client chose reaches a path. Without a request in front of it, naming the walk does what every member of
`Core\Request` does:

```nvs
<?nvs
try {
    Core\Request::files();
    echo "not reached\n";
} catch (LogicError $none) {
    echo "no request here\n";
}
```
```output
no request here
```
