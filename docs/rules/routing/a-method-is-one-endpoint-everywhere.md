A method whose repeated `#[Route]`s serve several verbs is one endpoint everywhere it is observed:

- **`Core\Router::url(App::receive)` returns the one path**, because every `#[Route]` on one method
  shares its `path` (`rule:routing/routes-are-compiled-not-registered`), which makes method-to-path a
  function. No verb argument is added to `url` or `urlAbsolute`; a caller that needs to know which
  verb to send already knows.
- **`Core\Router\Match::is(App::receive)` is `true` for every verb the method serves**, and
  `Match::method()` tells the verbs apart where that matters
  (`rule:routing/a-match-is-not-invocable`).
- **The generated `operationId` is `Class.method`, with the lowercased verb appended when the method
  serves several verbs** — `App.receive.post`, `App.receive.put`. OpenAPI requires the id to be unique
  per operation, and `(path, method)` is unique by the duplicate-route rule, so the suffix is
  deterministic and needs no counter. A method that serves one verb is emitted bare. `#[Api]`'s
  `operationId` replaces the derived id
  (`rule:routing/api-document-is-generated-from-the-route-table`).
- **The metrics `route` label is the one path pattern for every verb**
  (`rule:observability/route-label-is-the-path-pattern`), so a dashboard shows one series per endpoint.

The route table's shape does not change: one row per `(path, method)`, each carrying its handler, and
the reverse index `url()` reads is keyed by the handler. Runtime cost is zero: a written reference
folds to the row index while compiling.
