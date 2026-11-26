`nvs debug` serves the debug viewer, a page in your browser that shows what your server recorded.

Turn on `[debug] store = true` in `nvs.toml` first. Then open the link that `nvs debug` prints, or
that `nvs debug link` prints again. The link contains a token, and the viewer opens only in a
browser that has it. The first page lists the newest requests, with filters. Open a request to see
its handler, its memory peak, its timeline, its queries, its dumps and its log lines. An exception
shows its stack. The Runs page lists scheduled jobs, queue jobs and runs of `nvs run`. Other pages
show totals for each route and each query.

A `file:line` in the viewer opens the file in your editor. Press `?` to see every key.

**Good to know:** `r` sends a recorded request to your server again. The handler runs again, with
all its side effects.

**The examples below** show three routes and what the viewer shows for each: a dump, the same query
that runs once for each order, and an error that is counted on the exceptions page.
