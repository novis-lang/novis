The interface your class implements to write the response from the value that a route method returns.

`Core\Router\Responder` extends `Core\Router\Gate`, so your class has the three `Gate` methods and
one more: `respond($route, $value)`. When a route method returns a value, `Core\Router::dispatch`
calls `respond` with that value. Your `respond` method writes the response, for example with
`Core\Response::html` or `Core\Response::json`.

A route method declared `void` does not return a value, so `dispatch` does not call `respond`. A
method declared `?T` that returns `null` still reaches `respond` with `null`.

**Good to know:** `allows` still runs first. A refused request and a route method that throws an
error never reach `respond`. If `respond` throws an error, `dispatch` throws the same error.
`nvs check` gives a warning when your gate is not a `Responder` and a route method returns a value.
