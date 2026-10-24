The compiler records each `Core\Request::jsonAs<T>()` and `Core\Request::postAs<T>()` call written
directly in a route handler's body, and the route's row carries what it found: the body is `T`, as JSON
for `jsonAs` and as a form for `postAs`. The API document writes it as the operation's `requestBody`,
with `T`'s schema under `application/json` or `application/x-www-form-urlencoded`
(`rule:routing/api-document-is-generated-from-the-route-table`). A JSON body is `required`, because
`jsonAs` throws on an empty body. A form body is `required` when `T` has a required field, because
`postAs` throws when one is missing.

There is no new syntax. The type is the one the handler already writes to decode its body, so the
document cannot name a body the code does not decode (`rule:attributes/api-adds-and-cannot-contradict`).
A handler with a `jsonAs` and a `postAs` gets both media types. Two reads of one kind with different
types give that media type a `oneOf` of the two schemas. A `postAs` call that writes its `name` option reads
one field of the form and not the whole body, so the document does not take a body from it.

A body read inside a helper method is not seen. A handler that reaches a `jsonAs` or `postAs` call only
through methods it calls gets no `requestBody`, and `nvs check` warns at the route that the document does
not describe its body. The warning follows the calls the checker resolved, through any number of
methods, and does not follow a call through a `callable` or a `mixed` receiver. Moving the read into the
handler removes the warning.

- **Memory:** one body entry per route row, on the compile path only. The warning's walk runs only in
  `nvs check` over a program with routes, and holds one list of resolved callees per method while it
  runs.

**Declined:**
- **An `#[Api(body: ...)]` field.** It could name a type the handler does not decode, the contradiction
  `rule:attributes/api-adds-and-cannot-contradict` forbids.
- **A body parameter the server decodes before the handler runs.** It is new syntax and a second way to
  read a body.
- **Following a helper's body read into the document.** The published API would then change when a read
  moves between two methods, without any change to the handler.
