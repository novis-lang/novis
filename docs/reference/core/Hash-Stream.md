---
summary: a digest of data your program makes in pieces. You add each piece with `update` and read the digest once with `finish`
keywords: incremental hash, streaming digest, chunked input, checksum
---

`Core\Hash::stream` opens a `Core\Hash\Stream` with a `Core\Hash::Digest` case. Each `update` adds
one piece of data, in order. `finish` returns the same digest that `Core\Hash::of` returns for all
the pieces joined together. After `finish`, the stream is closed. A second `finish` or another
`update` throws a `RuntimeError`, so two digests of one input need two streams.

To hash a file, an upload or the request body, pass it to `Core\Hash::of`. It reads the data in
pieces for you.

```nvs
<?nvs
var $stream = Core\Hash::stream(Core\Hash::Digest::Sha256);
$stream->update('a');
$stream->update('bc');
bytes $streamed = $stream->finish();
echo Core\Encoding::toHex($streamed), "\n";

bytes $whole = Core\Hash::of('abc', Core\Hash::Digest::Sha256);
echo Core\Hash::equals($streamed, $whole) ? 'same' : 'differ', "\n";

try {
    $stream->update('more');
} catch (RuntimeError $e) {
    echo "finished\n";
}
```
```output
ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
same
finished
```
