Calls the method of the route that the request matched. You call `Core\Router::dispatch` once, in
the file that answers requests. You give it your gate: an instance of a class that implements
`Core\Router\Gate`.

The gate has three methods. `allows` runs before each route method, and the method runs only when it
returns `true`. `refused` writes the page when no route method runs, such as a missing page. `failed`
writes the page when the route method throws an error. `dispatch` sets the status first, so the
right code is sent.

A capture is a value from the path, such as `42` in `/articles/42`. Each capture and `#[Core\Query]`
value is converted to its parameter's type before `allows` runs. `allows` reads them with
`$route->parameter()`.

**Good to know:** if `allows` throws an error, the route method does not run. When a route method
returns a value, a gate that implements `Core\Router\Responder` sends it with `respond`. A plain gate
does not use the value, and `nvs check` gives a warning.

**The examples below** show two routes, a gate that checks a login, and an error page.
