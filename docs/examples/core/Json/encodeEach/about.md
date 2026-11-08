Writes a JSON array in pieces, so a long list never has to be in memory as one string.

`Core\Json::encodeEach` takes an array or an iterable, such as the rows of a database query. It
returns pieces of at most 64 KiB that you read with a `foreach` loop. It takes the next item only
when it needs it.

Each item is encoded as `Core\Json::encode` encodes it. The pieces are `bytes`. If an item can
contain tainted text, such as a value from a request, the pieces are `tainted bytes`.

Set `lines` to `true` to write JSON Lines, with one value on each line.

To send the pieces as a response, pass each one to the `write` method of
`Core\Response::stream('application/json')`. `write` does not accept tainted pieces. To write the
pieces to a file, use `Core\IO::writeStream($path, $pieces)`.

**Good to know:** if an item throws an error halfway, your loop may already have received some
pieces.
