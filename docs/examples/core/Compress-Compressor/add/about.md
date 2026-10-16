Adds one piece of data to a compression stream you opened with `Core\Compress::compressor`.

You can call `add` as many times as you need, and each piece is a `string` or `bytes`. Nothing is
compressed yet. The stream keeps the pieces in the order you added them, and `finish` compresses all
of them together into one block. The result is the same however you split your data up.

`add` keeps the piece you gave it. It does not copy the data, so adding the same large piece a
thousand times uses the memory of one piece.

**Good to know:** a stream ends once. After `finish`, `add` throws a `RuntimeError`, and you open a
new stream for the next block of data.
