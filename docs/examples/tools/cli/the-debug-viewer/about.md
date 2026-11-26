`nvs debug` serves the debug viewer, a page in your browser that shows the requests your server
recorded.

Turn on `[debug] store = true` in `nvs.toml` first. Then open the link that `nvs debug` prints, or
that `nvs debug link` prints again. The link contains a token, and the viewer opens only in a
browser that has it. The first page lists the newest requests, with filters. Each row shows the
status, the duration, and how much of that time the queries took. Open a request to see its
timeline, its request and response, its queries, its `Core\Debug::dump` values and its log lines,
each on its own tab. Other pages show totals for each route and each query.

A `file:line` in the viewer opens the file in your editor. Press `?` to see every key.

**Good to know:** `r` sends a recorded request to your server again. The handler runs again, with
all its side effects.

**The examples below** show three routes and what the viewer shows for each: a dump, the same query
that runs once for each order, and an error that is counted on the exceptions page.
