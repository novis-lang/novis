Returns the mount that is serving this request, as a `Core\Request\Mount`. A mount is one entry
in the server's configuration. It says which program answers the addresses that start with a
prefix, such as `/shop`.

The server removes the prefix from the path before your program sees it. `prefix()` returns the
removed text, so you can put it in front of the links you build. A mount at `/` removes nothing,
and `prefix()` returns `""`.

A prefix can contain a pattern that matches many folders, one for each customer. `captures()`
returns the parts that the pattern matched, in order. These strings are tainted. A tainted
string came from the client, so Novis makes you check it before you use it in a query or a
file path.

A command-line program answers no request, so `mount` throws a `LogicError` there.

**The examples below** show a link built under the prefix, a program served at the root, and one
program that serves many customers.
