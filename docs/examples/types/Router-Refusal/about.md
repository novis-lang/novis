The reason `Core\Router::dispatch` did not call the method of a route.

`dispatch` sets the status, then passes the reason to the `refused` method of your
`Core\Router\Gate`. Your gate writes the body, and it can change the status.

| Case | Status | When |
|---|---|---|
| `NotFound` | `404` | No route has this path. |
| `MethodNotAllowed` | `405` | A route has this path, but not this HTTP method. |
| `UnknownMethod` | `501` | The HTTP method is not a case of `Core\Http\Method`. |
| `Denied` | `403` | `allows()` returned `false` or threw an error. |
| `Csrf` | `403` | The CSRF token or the origin is not accepted. |
| `BodyTooLarge` | `413` | The body is larger than the server's limit. |
| `BadCapture` | `400` | A path or query value does not convert, or a required query key is missing or repeated. |

`route` is `null` when no route matched.

A gate can set `404` for both `BadCapture` and `Denied`. Then a visitor cannot find out which ids
exist.
