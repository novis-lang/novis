Returns the URLs of other servers that the directory named for a search, as an array of text.

A directory can keep some of its parts on other servers. When a search reaches such a part, the
server does not send its entries. It sends a URL instead, called a reference. `references` returns
these URLs in the order the server sent them. Read them after the loop, because the list is only
complete then. Most searches stay on one server, and then the array is empty.

**Good to know:** Novis never connects to these servers by itself. Every URL is `tainted`, because
the server sent it. Check a URL before you use it, for example to open another connection.
