Returns the path of the request that your program is answering, such as `/orders/1042`. The
path is the part of the address after the host name and before the `?`.

The server can serve your program under a prefix, such as `/shop`. The server removes that
prefix before your program runs, so `path` returns `/cart` for the address `/shop/cart`. Your
program then reads the same paths under any prefix. `Core\Request::mount` returns the prefix
that was removed.

The path is returned as the client sent it. A space is still written as `%20`. The result is
tainted. A tainted string came from the client, so Novis makes you check it before you use it
in a query or a file path. A command-line program answers no request, so `path` throws a
`LogicError` there.

**The examples below** show a path with a `%20` in it, a program served under a prefix, and a
program that picks a page by its path.
