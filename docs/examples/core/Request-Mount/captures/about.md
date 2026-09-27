Returns the parts of the mount's prefix that a pattern matched, as an array of strings.

A server can use one pattern to serve many folders, one for each customer. A pattern such as
`/{1:lower}` serves the folder `Acme` at `/acme` and the folder `Globex` at `/globex`. When a request
arrives at `/acme`, `captures()` returns `["Acme"]`. The first part, `{1}`, is `captures()[0]`, the
second part is `captures()[1]`, and so on. A mount without a pattern returns an empty array.

The strings are tainted. A tainted string came from the client, so Novis makes you check it before
you use it in a query or a file path.

**In plain words:** the address is a door, and the capture is the name written on it. The client
picks which door to knock on, so your program checks the name before it trusts it.

**The examples below** read the parts of a pattern in order, handle a mount without a pattern, and
pick the language of a page from the address.
