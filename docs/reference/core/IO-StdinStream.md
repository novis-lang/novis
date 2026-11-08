---
summary: the program's standard input, read in pieces. `Core\IO::stdinStream` returns it. You read it with a `foreach` loop
keywords: read stdin, standard input, stdin in chunks, pipe input, large input, binary input, checksum of input
---

`Core\IO\StdinStream` is the program's standard input, read in pieces. `Core\IO::stdinStream`
returns it. The class has no methods. You read it with a `foreach` loop.

Each item is the next piece of the input, up to 64 KiB, as `tainted bytes`. The loop ends when the
input ends. Only one piece is in memory at a time, so a very large input works too. The bytes are
not changed, so input that is not UTF-8 text gives no error. The input can be read only once.

`Core\Hash::of` accepts a `Core\IO\StdinStream`, so one call computes a checksum of the whole input.

```nvs skip
<?nvs
uint $total = 0;
foreach (Core\IO::stdinStream() as tainted bytes $piece) {
    $total = $total + Core\Bytes::length($piece);
}
echo 'bytes: ', $total, "\n";
```
