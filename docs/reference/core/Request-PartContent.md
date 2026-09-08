---
summary: one uploaded part's bytes as a walk over its chunks — `Core\Request\Part::content()`'s answer, holding one chunk at a time and valid only while its part is the current one
keywords: streaming upload, chunk, fread, stream_get_contents, multipart/form-data, large file upload, memory_limit, Iterable bytes, tmp_name, move_uploaded_file
---

`Core\Request\PartContent` is what `Core\Request\Part::content()` answers with: an `Iterable<tainted
bytes>` that a `foreach` walks once over one uploaded part. It has no members — the walk is everything it
does — and it holds nothing but the chunk the loop body is looking at, so a part far larger than the
machine's memory passes through a program that never has more than one chunk of it resident. It is the
reading for an upload that must never be whole in memory, and `readAll` and `saveTo` are both written
over it: those two are the bounded and the on-disk answers to the same question, and this is the one
where the program decides what to keep.

**A part is a position in a body, and this walk is only valid while it is the current one.** The `files()`
parse moves whether or not this walk is what moved it, so the part the walk was named on is remembered
and checked on every pull: once the loop over `files()` has advanced, a pull here refuses with
`LogicError` rather than handing back a later part's bytes. That is a defect in the program — a name kept
past the iteration that opened the next part — and never something a peer can provoke.

**A chunk boundary is the wire's, and carries no meaning.** It is where the octets happened to arrive, so
a program that needs lines, records or frames finds them across chunks; nothing here aligns a chunk to
anything the sender wrote. The walk is empty for a part that carried no bytes, which is an ordinary thing
for a form to send.

Every chunk is `tainted bytes`, like everything else a peer chose, so it reaches a sink only through a
launderer named for that sink — and the filename the part carries is tainted too, which is why a path is
built rather than concatenated. Where the destination is simply disk, `saveTo` is the shorter spelling of
this loop and bounds itself; reach for `content()` when the bytes are being counted, hashed, decoded or
forwarded rather than stored.

```nvs skip
<?nvs
foreach (Core\Request::files() as Core\Request\Part $part) {
    int $seen = 0;
    foreach ($part->content() as tainted bytes $chunk) {
        $seen = $seen + Core\Bytes::length($chunk);
    }
    echo $part->name(), ": ", $seen, " bytes\n";
}
```

Beyond the `LogicError` above, the walk carries the failures the body itself has: `IOError` where the
connection failed underneath it or the peer stopped short of the closing boundary, and `ParseError` where
what arrived is not the multipart body the request declared. Reaching the walk at all needs a request in
front of it, and without one the entry to it refuses the way every member of `Core\Request` does:

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
