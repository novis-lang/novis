Adds one piece of compressed data to a stream you opened with `Core\Compress::decompressor`.

You can call `add` as many times as you need, and each piece is `bytes`. Nothing is read yet. The
stream keeps the pieces in the order you added them, and `finish` decompresses all of them together.
This is how you read data that arrives in parts, such as a download or a request body.

`add` never looks inside the piece, so data that is not compressed at all is accepted here and
refused by `finish`. The size limit belongs to the whole stream as well: splitting the data into
more pieces does not allow a larger result.

**Good to know:** a stream ends once. After `finish`, `add` throws a `RuntimeError`, and you open a
new stream for the next download.
