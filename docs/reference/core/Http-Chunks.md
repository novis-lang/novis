---
summary: The body of a streamed reply as pieces of raw bytes. `Core\Http\Stream::chunks` returns it, and a `foreach` loop reads it
keywords: stream, streamed reply, streaming download, raw bytes, binary body, read a body in pieces, large download, download size limit, chunks
---

`Core\Http\Chunks` is the body of a streamed reply, as pieces of raw bytes.
`Core\Http\Stream::chunks` returns it. You read it with a `foreach` loop. Each piece is
`tainted bytes`, because another server sent it.

The pieces are not split at lines or at any other boundary. The size of each piece depends on the
network. Together, the pieces are the whole body, in order. Use `chunks()` for a body that is not
text, such as an image or an archive. For text with one record on each line, use
`Core\Http\Stream::lines`. For server-sent events, use `Core\Http\Stream::events`.

The body of a stream can be read only once. After `chunks()`, a call to `chunks()`, `lines()`,
`events()` or `saveTo()` on the same stream throws a `LogicError`. You can stop the loop with
`break`, and the rest of the body is not read. A second `foreach` over the same `Core\Http\Chunks`
continues after the last piece the first loop read.

A server that sends nothing for longer than the call's `idle` time throws a `TimeoutError` in the
loop. A body that takes longer than `maxDuration` throws one too. A connection that fails before
the body ends throws an `IOError`.

```nvs
<?nvs
// `Core\Test::answerHttp` gives a fake reply, so this example does not use the network.
Core\Test::answerHttp('https://files.example.com/report.csv', 200, {
    body: "date,sales\n2026-09-01,120\n2026-09-02,95\n",
});

Core\Http\Stream $download = Core\Http\Client::stream(Core\Http\Method::Get, 'https://files.example.com/report.csv');
uint $size = 0;
foreach ($download->chunks() as tainted bytes $piece) {
    $size = $size + Core\Bytes::length($piece);
    // This loop stops when the download is larger than 1 MB.
    if ($size > 1000000) {
        echo "the download is too large\n";
        break;
    }
}
echo 'received ', $size, " bytes\n";

// The body was already read, so a second call throws a `LogicError`.
try {
    $download->lines();
} catch (LogicError $error) {
    echo "the body cannot be read twice\n";
}
```
```output
received 40 bytes
the body cannot be read twice
```
