`nvs debug` serves the debug viewer, a page in your browser that shows what your server recorded.

Turn on `[debug] store = true` in `nvs.toml`. Then open the link that `nvs debug` prints. The link
contains a token, a long random text. The viewer opens only in a browser that opened this link. The
first page lists the newest requests. Open a request to see the method that handled it, the most
memory it used, its timeline, its queries, its dumps and its log lines. An exception shows its
stack. The Runs page lists scheduled jobs, queue jobs and runs of `nvs run`. Other pages show totals
for each route and each query.

A `file:line` in the viewer opens the file in your editor. Press `?` to see every key.

**Good to know:** `r` sends a recorded request to your server again. The handler runs again, so a
mail is sent again and a row is written again.

**The examples below** show three routes. One makes a dump. One runs a query again for each order
in a loop. One throws an error that the Exceptions page counts.
