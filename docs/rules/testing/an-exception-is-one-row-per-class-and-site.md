The exceptions view shows one row per exception class and throw site, with how many times it was
thrown, when it was first and last thrown, and links to the requests that threw it.

A site is the file and line of the `throw` that raised it, as the record's first frame names it. The
row shows the class, the message of the latest throw, the site as a link into the editor
(`rule:testing/a-site-in-the-viewer-opens-in-the-editor`), the count, the first and last time, a small
chart of the count over time and the routes it was thrown under. Opening a row lists its requests
newest first, and each opens to its timeline at the throw. The grouping is a column the ingester
writes with the record, so the view is one indexed query.

An edit that moves the `throw` to another line starts a new row. The rows describe the code that
ran, and the index keeps them only as long as its deletion window, so an old row ages out with its
requests.

- **Memory:** one group key per exception record, on disk in the index.

**Declined:**
- **Grouping by the throwing function instead of the line.** Two different `throw` statements of one
  class in one function would share a row, and the row could not say which one ran.
- **Marking a group resolved or ignored.** That is state an index rebuilt from the files would lose,
  and the deletion window already ends a row that stops recurring.
