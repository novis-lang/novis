---
summary: the request body as a walk over its chunks — `bodyStream()`'s answer, consumed once, holding one chunk at a time
keywords: php://input, fopen php://input, fread, stream_get_contents, streaming request body, large upload, chunked body, chunk, Iterable bytes, memory_limit
---

`Core\Request\BodyStream` is what `Core\Request::bodyStream()` answers with: an `Iterable<tainted bytes>`
that a `foreach` walks once. It has no members — the walk is everything it does — and it holds nothing
but the chunk the loop body is looking at, so a body far larger than the machine's memory passes through
a program that never has more than one chunk of it resident. That is the whole difference from `body()`,
which fills the request's hold under `[limits] request_body` and can be read back any number of times.
This walk is under no such cap, because what a program keeps out of it is the program's own decision and
its own bill.

**A chunk boundary is the wire's, and carries no meaning.** It is where the octets happened to arrive, so
a program that needs lines, records or frames finds them itself, across chunks; nothing here aligns a
chunk to anything the sender wrote. Each chunk is copied out as the loop body receives it and stays valid
after the next pull, so keeping one keeps bytes the wire cannot revoke. The walk is empty where the
request carried no body at all.

**Naming the walk is the reading.** `bodyStream()` claims the body when the walk is built, long before a
single byte moves, so `body()`, `post()`, `json()`, `jsonAs<T>()` or `files()` after it is refused with
`LogicError` whether or not the loop ever ran — and refused the same way in the other direction. That
refusal is a defect in the program, never something a peer can provoke; the split between the readers
that share the body and the ones that consume it is
`rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`, and `Core\Request`
is where all five readers are named together.

Every chunk is `tainted bytes`, like everything else a peer chose, so it reaches a sink only through a
launderer named for that sink. The common destination is disk, and `Core\IO::writeStream` takes this walk
directly — it is where every stream reaches a file, under a byte bound of its own, and it removes the
partial file if a write fails, so there is no half-written upload to clean up after.

```nvs skip
<?nvs
int $total = 0;
foreach (Core\Request::bodyStream() as tainted bytes $chunk) {
    $total = $total + Core\Bytes::length($chunk);
}
echo "read ", $total, " bytes\n";
```

Or spent on a file instead, where the bytes never become a value the program holds. One request carries
one body, so this is the *other* program, not the next few lines of that one:

```nvs skip
<?nvs
Core\IO::writeStream("incoming.bin", Core\Request::bodyStream(), {max: 64 * 1024 * 1024});
```

Both of those need a request in front of them. Without one, naming the walk is what every other member of
`Core\Request` does:

```nvs
<?nvs
try {
    Core\Request::bodyStream();
    echo "not reached\n";
} catch (LogicError $none) {
    echo "no request here\n";
}
```
```output
no request here
```
