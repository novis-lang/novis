The interface your class implements to control `Core\Router::dispatch`.

You pass an instance of your class to `Core\Router::dispatch`. The class has three methods:

| Method | When `dispatch` calls it |
|---|---|
| `allows($route)` | Before every route method. Return `true` to run the method. |
| `refused($reason, $route)` | When no route method runs. `$reason` is a `Core\Router\Refusal`. |
| `failed($route, $error)` | When the route method threw an error. The status is `500`. |

When `allows` returns `false`, the status is `403` and `dispatch` calls `refused` with
`Refusal::Denied`. An error thrown in `allows` has the same result.

Before `dispatch` calls `refused` or `failed`, it sets the status of the response. Your method writes
the body. It can also set another status with `Core\Response::setStatus`.

**Good to know:** if `refused` or `failed` throws an error, `dispatch` throws the same error.
