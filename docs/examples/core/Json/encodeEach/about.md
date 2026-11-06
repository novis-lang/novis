Writes a JSON array in pieces, so a long list never has to be in memory as one string.

`Core\Json::encodeEach` takes an array or an iterable, such as the rows of a database query. It
returns pieces of text that you read with a `foreach` loop. Each piece is at most 64 KiB, and the
`[`, `,` and `]` are part of the pieces. The function takes the next item only when it needs it.

Each item is encoded the same way `Core\Json::encode` encodes it, and throws the same errors. The
pieces are tainted bytes.

Set `lines` to `true` to write JSON Lines. Each value is then on its own line, with no brackets.

Write the pieces to a file with `Core\IO::writeStream($path, $pieces)`, or `echo` each piece in a
`foreach` loop. Pass `{ overwrite: true }` to replace a file that already exists.

**Good to know:** if an item throws an error halfway, your loop may already have received some
pieces. The text of earlier items that was not yet in a piece is lost.
