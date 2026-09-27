Returns the method of the request, such as `GET` or `POST`, as a case of the enum
`Core\Http\Method`. You compare it with a case: `Core\Request::method() == Core\Http\Method::Post`.

The method says what the client wants to do. `GET` reads a page, `POST` sends a form, `PUT` and
`PATCH` change something, and `DELETE` removes it. One address often does different things for
different methods.

A `HEAD` request returns `Get`, because Novis runs it the same way as a `GET` request. Use
`Core\Request::isHead` when you need to tell the two apart.

A command-line program answers no request, so `method` throws a `LogicError` there.

This replaces `$_SERVER['REQUEST_METHOD']` in PHP.

**The examples below** show a form that is shown or saved, a page that answers `405` for a
method it does not support, and one address that reads, changes and deletes a record.
