The interface your class implements to control `Core\Router::dispatch`. You pass an instance of the
class to `dispatch`, and it calls three methods:

| Method | When `dispatch` calls it |
|---|---|
| `allows($route)` | Before every route method. Return `true` to run the method. |
| `refused($reason, $route)` | When no route method runs. `$reason` is a `Core\Router\Refusal`. |
| `failed($route, $error)` | When the route method threw an error. The status is `500`. |

When `allows` returns `false` or throws an error, the status is `403` and `refused` gets
`Refusal::Denied`.

Every capture and `#[Core\Query]` value is converted before `allows` runs, and
`$route->parameter()` returns it with its type. So `allows` can check a record that a `parse` method
loaded.

`dispatch` sets the status before it calls `refused` or `failed`. Your method writes the body, and
`Core\Response::setStatus` can change the status.

**Good to know:** an error in `refused` or `failed` is thrown again by `dispatch`. To send the value
a route method returns, implement `Core\Router\Responder`. Otherwise `nvs check` gives a warning.
