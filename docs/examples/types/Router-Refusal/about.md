The reason `Core\Router::dispatch` did not call the method of a route.

`dispatch` passes it to the method `refused` of your `Core\Router\Gate`. Before that call, it sets
the status of the response. Your gate writes the body, and it can also set another status.

| Case | Status | When |
|---|---|---|
| `NotFound` | `404` | No route has this path. |
| `MethodNotAllowed` | `405` | A route has this path, but not for this HTTP method. The `Allow` header lists the methods that the path accepts. |
| `UnknownMethod` | `501` | The HTTP method is not a case of `Core\Http\Method`. |
| `Denied` | `403` | `allows()` returned `false` or threw an error. |
| `Csrf` | `403` | The server did not accept the CSRF token or the origin of the request. |
| `BodyTooLarge` | `413` | The request body is larger than the limit of the server. |
| `BadCapture` | `400` | A value in the path or in the query string cannot be converted to the type of its parameter. A required query key is missing, or a query key is written twice. |

`route` is `null` when no route matched the request.
