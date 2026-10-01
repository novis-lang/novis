Returns the name of an HTTP method as text. `Core\Http::methodName` takes a case of
`Core\Http\Method` and returns its name in capital letters, the way it is written in a request.
`Core\Http\Method::Get` gives `GET`, and `Core\Http\Method::Delete` gives `DELETE`.

Use it when you need the method as a string. Two common cases are the `Allow` header of a
`405 Method Not Allowed` response and a log line. `Core\Router::methodsFor` and
`Core\Request::method()` return `Core\Http\Method` cases, so you pass their result to this method.

The method never throws an error. Each of the eight cases has a name.

The examples print the name of every method, build an `Allow` header, and write the method of the
current request into a log line.
