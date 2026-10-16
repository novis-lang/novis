Returns every line of one header of a streamed reply, as a list.

A server can send the same header on more than one line. `headers()` returns one `tainted string`
for each line, in the order they arrived. A header that was sent once gives a list with one entry.
A header that was not sent gives an empty list. The name can be written in any case.

`header()` joins the lines into one text. Use `headers()` when the lines must stay apart. This is
always the case for `set-cookie`, because one cookie can contain a comma. For this reason
`header("set-cookie")` throws a `LogicError`, and `headers("set-cookie")` works.

**The examples below** read every cookie of a reply, count the lines of three headers, and build
the `cookie` header for the next request.
