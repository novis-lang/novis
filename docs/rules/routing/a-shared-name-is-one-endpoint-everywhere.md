A route whose repeated attributes share a `name` is one thing everywhere it is observed:

- **`Core\Router::url("webhook", […])` returns the one path**, because the shared-name condition
  (`rule:routing/repeated-routes-share-a-name-when-they-share-a-path`) makes name-to-path a
  function. No verb argument is added to `url` or `urlAbsolute`; a caller that needs to know which
  verb to send already knows.
- **`Core\Router\Match::name` carries the shared name for every verb**, so the observability
  `route` label reports one series per endpoint rather than one named series plus a heap of
  unlabelled requests, which looks like a working dashboard and is not one.
- **The generated `operationId` is the `name`, with the lowercased verb appended when two
  operations share it** — `webhook.post`, `webhook.put`. OpenAPI requires the id to be unique per
  operation, and `(path, method)` is unique by the shared-name condition, so the suffix is
  deterministic and needs no counter. A name carried by exactly one operation is emitted bare, so no
  existing document moves; a route that declared no name falls back to its `Class::method` handler
  label, which is unique by construction.

The route table's shape does not change: one row per `(path, method)`, with `name` a column on each
row rather than a key into them, and the reverse index `url()` reads is built from that column.
Runtime cost is zero and memory cost is one `?string` column's worth of repeats.
