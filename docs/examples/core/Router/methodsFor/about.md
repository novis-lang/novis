Lists the HTTP methods that the routes of a path accept. `Core\Router::methodsFor` checks the path
against every `#[Route]` in your program and returns an array of `Core\Http\Method` values. Each
method is in the array once, in the order the routes are declared.

Use it when a request matches no route, to choose the status of the response. An empty array means
that no route has this path, so the status is `404 Not Found`. A non-empty array means that the
path exists for other methods, so the status is `405 Method Not Allowed`, and the array is the list
for its `Allow` header.

A capture that does not fit its handler parameter does not match. For example, `abc` in the place
of a `uint $id` gives an empty array. The method never throws an error.

The examples list the methods of one path, show a path that no route has, and choose between a
`404` and a `405`.
