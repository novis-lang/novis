`Core\Secret::reveal()` returns the text of a `secret string` as a plain `string`.

A value with the type `secret string` is a password, a token or another text that must stay private.
Your program can pass it to functions that are written for secrets, such as a password check. It
cannot print it, write it to a log, put it in JSON or send it anywhere else. When one line of your
program really must use the text, you call `reveal()` on that line.

Every call needs a second argument: a short reason, written for the people who read the code. The
program never reads it, so it never appears in the result. `reveal()` removes `secret` and nothing
else. A value that was also `tainted` stays `tainted`.

**In plain words:** a sealed envelope. You can carry it around freely, and opening it is one visible
step with a note that says why.

**The examples below** show a new token printed once, a key shown with only its last four characters,
and a token placed in the header of a web request.
