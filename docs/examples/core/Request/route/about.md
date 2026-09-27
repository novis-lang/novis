Returns the route that the request matched. A route is a method of your program with a
`#[Core\Route]` attribute, such as `#[Core\Route(path: "/users/{id}", method: Core\Http\Method::Get,
name: "Users::show")]`. The server compares the request with every route once, before your program
starts. `route()` returns the result of that comparison, so your program does not compare the path
again.

The result is a `Core\Router\Match`. Its method `name()` returns the name of the route, and
`param("id")` returns the value of the part `{id}` in the path. That value already has the type
that the handler declares, so `/users/42` gives the number `42` for a `uint $id`.

The result is `null` when no route matches the method and the path, and also when the program
declares no routes. A command-line program answers no request, so `route()` throws a `LogicError`
there. When a captured part decodes to bytes that are not valid UTF-8 text, `route()` throws a
`RuntimeError`.

**The examples below** read a matched route, show an address that no route matches, and mark the
open page in a menu.
