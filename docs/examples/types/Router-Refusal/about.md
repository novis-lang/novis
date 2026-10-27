The reason `Core\Router::dispatch` did not call the method of a route.

`dispatch` passes it to the method `refused` of your `Core\Router\Gate`. It sets the status of the
response first. Your gate writes the body, and it can change the status.

| Case | Status | When |
|---|---|---|
| `NotFound` | `404` | No route has this path. |
| `MethodNotAllowed` | `405` | A route has this path, but not this HTTP method. The `Allow` header lists the methods it accepts. |
| `UnknownMethod` | `501` | The HTTP method is not a case of `Core\Http\Method`. |
| `Denied` | `403` | `allows()` returned `false` or threw an error. |
| `Csrf` | `403` | The CSRF token or the origin is not accepted. |
| `BodyTooLarge` | `413` | The body is larger than the server's limit. |
| `BadCapture` | `400` | A path or query value does not convert to its type. Or a required query key is missing or repeated. |

`route` is `null` when no route matched.
