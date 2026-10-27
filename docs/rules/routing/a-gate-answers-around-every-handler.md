`Core\Router::dispatch(Core\Router\Gate $gate): void` calls the method of the route the server matched,
and the gate the program passes answers every request that method does not run for.

```nvs
interface Core\Router\Gate {
    public function allows(Core\Router\Match $route): bool;
    public function refused(Core\Router\Refusal $reason, ?Core\Router\Match $route): void;
    public function failed(?Core\Router\Match $route, ?Throwable $error): void;
}

interface Core\Router\Responder extends Core\Router\Gate {
    public function respond(Core\Router\Match $route, mixed $value): void;
}
```

The compiler generates the jump from the route table it already builds, one entry per route, indexed
by the matched row, so no program writes or keeps a list of its routes. The jump is code in the unit
and never a value: the handler's `Class::method` label still does not cross into the program, and the
match stays not invocable (`rule:routing/a-match-is-not-invocable`). **A handler may declare any return
type, and a returned value reaches the response only through the program's own `respond`.** When the
handler's declared return type is neither `void` nor `never` and the gate implements
`Core\Router\Responder`, `dispatch` calls `$gate->respond($route, $value)` once the handler returns,
then releases the value; a `?T` handler that returns `null` still reaches `respond` with `null`. The
gate writes the response, as `Core\Response::html` does for a `Core\Html\Markup`. A plain gate drops
the value, and so does a gate that declares a `respond` method without implementing `Responder`: the
name alone never decides it. A handler that throws reaches `failed` and never `respond`, a refused
request reaches neither the handler nor `respond`, and an error `respond` throws is thrown by
`dispatch`. Either way the declared type is what `nvs build --openapi` describes as the response body
(`rule:routing/api-document-is-generated-from-the-route-table`). **`nvs check` warns at a `dispatch`
whose gate cannot be a `Responder` while a handler returns a value** (`W1025`), at the gate argument,
naming up to three such handlers in load order and then how many more. "Cannot be" is over every
non-abstract class the program declares: the argument's static type must admit none that implements
`Responder`, and a type naming no class of the program, such as `Core\Router\Gate` or `mixed`, admits
all of them. So the warning is never wrong about the program as written, and a program with a
`Responder` that passes a plain gate through a wide type gets none. It is never an error: a program
that wants the value dropped writes a `respond` that does nothing, or declares the handler `void`.
A handler is a static method or a
method of a class with a no-argument constructor; any other `#[Route]` method is a compile error at its
attribute, because the jump has no arguments to build an instance with.

**`allows` runs before every handler, and the handler runs only when it returns `true`.** A `false` or
a throw is `Refusal::Denied`. This is the dispatch site that reads `#[Access]` on every route
(`rule:security/access-is-checked-for-presence-not-meaning`): a route cannot reach its handler around
it, because the jump is the only call to the handler `dispatch` makes.

**`refused` answers what the table or the server refused, and never overturns it.** `dispatch` sets
the status before it calls `refused`, so a gate that only renders a page sends the right code, and the
gate may set another status, headers, the body and a log line. The handler never runs. The cases of
`Core\Router\Refusal`:

| Case | Status | When |
|---|---|---|
| `NotFound` | `404` | no route claims the path |
| `MethodNotAllowed` | `405`, with `Allow` from `Core\Router::methodsFor` | the path exists under other verbs (`rule:routing/a-refused-verb-is-not-a-missing-path`) |
| `UnknownMethod` | `501` | the verb is none of `Core\Http\Method`'s cases |
| `Denied` | `403` | `allows` returned `false` or threw |
| `Csrf` | `403` | the server refused the request's CSRF token or origin (`rule:security/csrf-is-on-by-default`) |
| `BodyTooLarge` | `413` | the body is over the cap the server enforces, or a handler read a body over `[limits] request_body` and then threw |
| `BadCapture` | `400` | a capture whose type implements `Parses` threw from `parse` |

A server refusal added later is a new case. A body read that stops at `[limits] request_body` throws
`RuntimeError` and is recorded on the request, so a handler that throws after it reaches `refused`
with `BodyTooLarge`: the size is the client's fault, and a handler that caught only `ParseError`
would otherwise answer it `500`. A handler that catches the throw and returns answers as it chose.
**`failed` answers every other handler that threw,** and a program
stopped by its memory or CPU-time limit: that one runs again in a fresh isolate under the same limits,
with no body, and `failed` is told no route and no error, because the first run's isolate is gone and
a limit is not a throw. If the second run fails too, the answer is the empty `500` a failing program
has always sent, and nothing runs a third time. `dispatch` sets `500` before it calls `failed`.

**Refusal mode is how the server's own refusals reach the gate.** The door decides exactly as it does
for any program. For a request it refused for CSRF, for its body size or for an unknown verb, it then
runs the entry anyway: `Core\Request::route()` returns `null`, so neither `dispatch` nor a dispatcher a
program wrote can reach a handler; the request has no body; and `dispatch` reads the refusal off the
request and calls `refused`. A gate that throws there leaves the empty answer the door would have sent.
The door chooses the mount before it checks the verb, which moves nothing else because a mount is
chosen by path, and the in-process door does the same, so `Core\Test::request` exercises it.

**Only a program that calls `Core\Router::dispatch` gets refusal mode.** The compiler sees the call, as
it sees a link (`rule:routing/link-target-and-params-are-checked`), and marks the unit. A program that
dispatches by hand keeps the empty answers, because it would read a refused request as an ordinary one
with no route. `Core\Request::route()` and `Core\Router::match` serve it unchanged.

Some refusals never reach a gate. A CORS preflight's body is never shown by a browser
(`rule:http-server/a-preflight-is-answered-before-any-code-runs`). Malformed HTTP has no request yet,
and an unreadable `X-Forwarded-For` is read by a proxy. A socket that is also an event stream has
already ended. The capacity and drain `503`s and a path no mount covers are answered where no code may
run, so their pages, when they exist, are static and come from the configuration.

- **Memory:** one generated jump entry per route, per program. Nothing per request beyond the isolate
  the request already has; a refused request that reaches the gate costs what any request costs. A
  dispatching program's request keeps its head until its run ends, which the door held anyway, and
  a run a limit stopped pays for one more isolate under the same ceiling. `respond` adds one bit per
  route row and one per class descriptor, per compiled unit, and nothing per request.
- **Latency:** `respond` is one more method call, only for a request whose handler returns a value and
  whose gate is a `Responder`. A `void` handler or a plain gate pays one bit test.
- **Usability and simplicity:** a page is a method that returns its markup, and a test calls it as one,
  for one more interface and one warning. Security is unchanged, because `allows` still runs before
  the handler. The warning costs the checker one walk over the program's classes per `dispatch` call.

**Declined:**
- **A dispatch `switch` every program writes.** It repeats every `#[Route]`, nothing checks it, and an
  arm that forgets a route serves nothing while one that forgets `accessAs` serves the route unguarded.
- **A gate that can overturn a refusal.** A refusal would then depend on code a reviewer has to read
  beside the route. The one opt-out from CSRF stays `csrf: false` on the route itself.
- **The server calling the matched method.** The program calls `dispatch`, and a program that does not
  call it is served exactly as before (`rule:routing/the-servers-match-dispatches-nothing`).
- **Refusal mode for every program.** A hand-written dispatcher would read a refused request as an
  ordinary request with no route.
- **A built-in return-value-to-response rule.** Which value becomes which response is a framework
  opinion, so `dispatch` hands the value to the program's own `respond` and decides nothing about it.
- **Constructor injection.** It is a framework opinion, and a handler built by a no-argument
  constructor needs none.
- **Every handler returns `void`.** The OpenAPI document reads a route's response schema and its
  `#[Api]` example from the return type, so the rule would remove both from every document.
