---
summary: an incremental digest — fed piece by piece with `update`, closed once with `finish`
keywords: incremental hash, streaming digest, chunked input, hash a large file, checksum
---

A `Core\Hash\Stream` is opened by `Core\Hash::stream` with a `Core\Hash::Digest` case and digests
everything `update` feeds it, in order, so `finish` answers exactly what `Core\Hash::of` answers
over the concatenation. `finish` consumes the stream: a second `finish`, or an `update` after one,
throws `RuntimeError`. Two digests of one input need two streams.

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
