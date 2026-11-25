The reason `Core\Router::dispatch` did not call the method of a route.

`dispatch` sets the status and passes the reason to the `refused` method of your
`Core\Router\Gate`. It writes the body and can change the status.

| Case | Status | When |
|---|---|---|
| `NotFound` | `404` | No route has this path. |
| `MethodNotAllowed` | `405` | A route has this path, but not this HTTP method. |
| `UnknownMethod` | `501` | The HTTP method is not a case of `Core\Http\Method`. |
| `Denied` | `403` | `allows()` returned `false` or threw an error. |
| `Csrf` | `403` | The CSRF token or the origin is not accepted. |
| `BodyTooLarge` | `413` | The body is larger than the server's limit. |
| `BadCapture` | `400` | A query value is not valid, or a required one is missing or repeated. Or a `Parses` class throws for a path value. |

A path value that is not a valid `uint` or `Date` gives `NotFound`.

`route` is `null` when no route matches.

To hide which ids exist, a gate sets `404` for both `BadCapture` and `Denied`.
