`Core\Test::request(...)` builds a request and runs it through the compiled route table and the real
middleware chain — no socket, no port, microseconds per test. What answers it is the program's own
entry, run as an isolate with the match already on its carrier, never the matched handler, which
nothing may invoke directly. **An in-process request may not be made from inside one**: the entry
answering it is the entry that asked, so a second would answer itself forever, and the refusal is at
the door rather than at a depth ceiling that would report an engine limit instead of the mistake.

The signature is `Core\Test::request(Core\Http\Method $method, string $path, {headers?:
array<string>, body?: string|bytes, mount?: string, captures?: array<string>}):
Core\Test\Response`. A `?` in `$path` starts the query. A `body` that is not given is no body, which
is not an empty one. `mount` is the prefix the request came in under, `""` for the root, and
`captures` is the list `Core\Request\Mount::captures()` reads back, `{1}` first. The two are one
mount row, so `captures` without a `mount`, or keyed by anything but position, is a `LogicError`:
the root has no pattern to capture.

The response is a `Core`-owned instance whose readings are members, so a status is `status()` and
never a property: `status(): uint`, `body(): string`, `header(string $name): ?string`, `headers():
array<array<string>>`, `cookies(): array<string>`, `json({maxDepth?: uint}): mixed` and
`jsonAs<T>({maxDepth?: uint}): T`. They read a request's answer the way `Core\Request`'s members of
the same names read the request: `header` matches any case and joins repeated lines with `, `,
`headers` keys lower-cased names to lists of lines, and `json`/`jsonAs` carry the same bag, default
and errors. `Set-Cookie` is the one field `header` refuses, as `Core\Http\Response::header` does,
because two cookies joined are neither; `cookies` reads it by name, the last value of a name
winning. The headers are the ones the program's answer carries — its `Content-Type`, or HTML for an
echo, then every header it declared — and not the ones the server adds for itself. A request whose
program fails answers `500`, no body and no headers, which is what the server sends. Nothing on the
response is `tainted`: it is the program's own output.

The synthetic request's parameters arrive **`tainted`**, exactly as a real request's would, so a
handler that forgets to launder fails its test rather than production.

`#[Test(server: true)]` binds a real listener for the cases that genuinely need the wire. The port
is the operating system's and the address is loopback, so two suites on one machine never collide
and no suite serves the program under test to a network. The listener is a **sibling** of the test's
isolate rather than a child — leftover work is read off the test's own task — and it is retired when
the test that asked for it has joined. It serves under the **default** policy rather than the tree's:
reading a deployment's configuration would make the test's subject the deployment.
