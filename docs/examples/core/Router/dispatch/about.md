Calls the method of the route that the request matched. You call `Core\Router::dispatch` once, in
the file that answers requests, and give it your gate: an instance of your class that implements
`Core\Router\Gate`.

The gate has three methods. `allows` runs before each route method, and the method runs only when
it returns `true`. `refused` writes the page when no route method runs, for example for a missing
page or a missing CSRF token. `failed` writes the page when the route method throws an error.
`dispatch` sets the status before it calls `refused` or `failed`, so a gate that only writes a page
sends the right code.

**Good to know:** the gate cannot let a refused request through. If `allows` throws an error, the
route method does not run.

**The examples below** show two routes and a missing page, a gate that checks a login, and an
error page for a method that fails.
