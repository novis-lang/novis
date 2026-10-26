`nvs debug` serves the debug viewer, a page in your browser that shows the requests your server
recorded.

Turn on `[debug] store = true` in `nvs.toml` first. Then open the address that `nvs debug` prints,
which is `http://127.0.0.1:8137/` by default. The first page lists the newest requests. Each row
has the status, the duration, the number of queries and dumps, and whether the request threw an
error. Open a request to see its timeline, its queries, its `Core\Debug::dump` values and its log
lines. The exceptions page has one row for each error class and the line that threw it, with a
count.

Every `file:line` in the viewer is a link that opens the file in your editor. Press `?` to see
every key.

**Good to know:** `r` sends a recorded request to your server again. The handler runs again, with
all its side effects.

**The examples below** show three routes and what the viewer shows for each: a dump, the same query
run once for each order, and an error that is counted on the exceptions page.
