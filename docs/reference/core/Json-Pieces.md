---
summary: the text of a JSON document in pieces, which `Core\Json::encodeEach` returns. You read them one at a time with a `foreach` loop
keywords: write large JSON, stream JSON, export JSON, JSON response, JSON Lines, NDJSON, write JSON to a file, low memory
---

`Core\Json\Pieces` contains the JSON text of a list, in pieces. `Core\Json::encodeEach` returns it.
It takes an array or an iterable, such as the rows of a database query. The class has no methods.
You read it with a `foreach` loop.

Each item is `tainted bytes`, at most 64 KiB long. The `[`, the commas and the `]` are part of the
pieces. With `{ lines: true }`, the text is JSON Lines: one value on each line, with no brackets. Use
`$piece as string` when you need the text of a piece.

Nothing is encoded until the loop starts. The method takes the next item only when it needs it, so
the whole document is never in memory at once. You can read the pieces only once. After the last
piece, a loop gives no items.

The loop throws a `LogicError` when an item cannot be written as JSON, as `Core\Json::encode` does.
An object whose class does not have `#[Core\Json\Derive]` is one example. Your loop may already have
received some pieces before the error. To write the pieces to a file, pass them to
`Core\IO::writeStream`.

```nvs
<?nvs
array<{ id: int, name: string }> $users = [
    { id: 1, name: 'Ada' },
    { id: 2, name: 'Ben' },
];

// A short list fits in one piece.
foreach (Core\Json::encodeEach($users) as var $piece) {
    echo $piece as string, "\n";
}

// `writeStream` writes the pieces to a file. This file has one user on each line.
string $path = Core\Path::join(Core\IO::temporaryDir(), 'users.jsonl');
Core\IO::writeStream($path, Core\Json::encodeEach($users, { lines: true }), { overwrite: true });
echo Core\IO::read($path);
```
```output
[{"id":1,"name":"Ada"},{"id":2,"name":"Ben"}]
{"id":1,"name":"Ada"}
{"id":2,"name":"Ben"}
```
