Returns the part of the path that the server removed before your program started, such as `/shop`.

The server's configuration says which program answers which prefix. When a request for
`/shop/cart` arrives at a program mounted at `/shop`, the server removes `/shop`, and
`Core\Request::path()` returns `/cart`. `prefix()` returns `/shop`. Put it in front of every link
and cookie path you write, so the same program works under any prefix.

The prefix starts with `/` and does not end with one. A program mounted at `/` has nothing removed,
and `prefix()` returns `""`. The prefix is not tainted, because the person who runs the server
wrote it, not the client.

**The examples below** read the prefix beside the path, read a prefix that a pattern matched, and
limit a cookie to this program.
