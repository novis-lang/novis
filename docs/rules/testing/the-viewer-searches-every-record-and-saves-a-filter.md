One search box searches the text of every request, dump, log line, query and exception in the index,
and any search can be saved under a name and opened again from a link.

The text is an SQLite FTS5 index the ingester fills as it commits each batch, so a search is one
indexed query and the viewer never scans records. FTS5 is compiled into the SQLite Novis already
bundles, so it adds no dependency. What is indexed is the record after
`rule:errors/record-transformations`, so a `secret` value is already a redacted node and is not
searchable, and a value the record elided is not either.

Words match as words, a quoted phrase matches as a phrase, and a field term narrows the search:
`status:500`, `method:POST`, `route:`, `kind:query`, `level:error`, `duration:>200ms` and
`trace:<id>`. A pasted `traceresponse` value is a `trace:` term. Results come newest first, are
grouped by the request they belong to, and open that request's timeline at the matching record. The
search is part of the view's link, so a search is shared by copying the address.

A saved filter is a name and a search. It is listed beside the views, opens with its own link, and is
kept in the viewer's settings file, outside the index, so discarding the index keeps it.

- **Memory:** the FTS5 index lives on disk beside the records it indexes, and grows with them under
  the spool's byte cap and the index's deletion window. A search holds one page of results.

**Declined:**
- **A regular-expression search.** It cannot use an index, so its cost grows with every record kept.
- **Searching a parameter value of a query.** Parameters are where request data sits, and the query's
  text with its placeholders already finds the call.
