---
summary: The body of a streamed reply, one line at a time. `Core\Http\Stream::lines` returns it, and a `foreach` loop reads it
keywords: stream, streamed reply, read a reply line by line, JSON Lines, NDJSON, CSV export, large export, log stream, line ending
---

`Core\Http\Lines` is the body of a streamed reply, one line at a time. `Core\Http\Stream::lines`
returns it. You read it with a `foreach` loop. Each line is a `tainted string`, because another
server sent it.

A line ends at `\n`, and a `\r` just before the `\n` is removed too. An empty line is an empty
string. The last line is given even when it has no line ending. Use `lines()` for text with one
record on each line, such as JSON Lines or a CSV export. For server-sent events, use
`Core\Http\Stream::events`. For a body that is not text, use `Core\Http\Stream::chunks`.

Only the current line is kept in memory, so the body can be larger than the memory your program may
use. A line longer than 64 KB throws a `RuntimeError`. A line that is not valid UTF-8 throws a
`RuntimeError` too. A server that sends nothing for longer than the call's `idle` time throws a
`TimeoutError`. A connection that fails before the body ends throws an `IOError`.

The body of a stream can be read only once. After `lines()`, a call to `lines()`, `chunks()`,
`events()` or `saveTo()` on the same stream throws a `LogicError`. A second `foreach` over the same
`Core\Http\Lines` continues after the last line the first loop read.

```nvs
<?nvs
// `Core\Test::answerHttp` gives a fake reply, so this example does not use the network.
// The first two lines end with "\r\n". The last line has no line ending.
Core\Test::answerHttp('https://shop.example.com/export/orders.csv', 200, {
    body: "id,total\r\n1,20\r\n2,15\n3,7",
});

Core\Http\Stream $export = Core\Http\Client::stream(Core\Http\Method::Get, 'https://shop.example.com/export/orders.csv');
Core\Http\Lines $lines = $export->lines();

// The first loop reads only the header line.
foreach ($lines as tainted string $header) {
    echo '[', $header, "]\n";
    break;
}

// This loop continues with the next line. Each line has the form "id,total".
int $sum = 0;
foreach ($lines as tainted string $line) {
    array<tainted string> $columns = Core\Str::split($line, ',');
    $sum = $sum + ($columns[1] as ?int ?? 0);
}
// This prints "total: 42".
echo 'total: ', $sum, "\n";
```
```output
[id,total]
total: 42
```
