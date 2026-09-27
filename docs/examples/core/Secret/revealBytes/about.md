`Core\Secret::revealBytes()` returns `secret bytes` as plain `bytes`.

It works like `Core\Secret::reveal()`, for binary data such as an encryption key. Keys that
`Core\Crypto` creates are `secret bytes`, so your program cannot print them, log them or save them by
mistake. When the program must save a key or send it somewhere, it calls `revealBytes()` on that
line, with a reason for the people who read the code.

The result is the same bytes, and every byte is kept, including bytes that are not valid text. A
value that was also `tainted` stays `tainted`.

**The examples below** measure a new key, turn a key made from a password into hex, and write a new
key into a line for a configuration file.
