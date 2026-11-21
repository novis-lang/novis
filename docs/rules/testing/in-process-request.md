`Core\Test::request(...)` builds a request and runs it through the compiled route table and the real
middleware chain — no socket, no port, microseconds per test. What answers it is the program's own
entry, run as an isolate with the match already on its carrier, never the matched handler, which
nothing may invoke directly. **An in-process request may not be made from inside one**: the entry
answering it is the entry that asked, so a second would answer itself forever, and the refusal is at
the door rather than at a depth ceiling that would report an engine limit instead of the mistake.

**The entry is the file a served request runs.** Under `nvs test` over a folder that has a
`public/index.nvs`, the file a tree with no `[[server.mount]]` serves, a request runs that file's
frame alone, so a script under `jobs/` or `bin/` that the folder's generated entry requires never
runs per request (`rule:testing/a-directory-of-programs-is-one-test-program`). Every other program
answers with its own entry. The `#[Test(server: true)]` listener still runs the generated entry.

The signature is `Core\Test::request(Core\Http\Method $method, string $path, {headers?:
array<string>, body?: string|bytes, json?: mixed, form?: array<string>, multipart?:
array<string|Core\Http\Part>, contentType?: string, cookies?: array<string>, clientIp?: string, scheme?:
string, mount?: string, captures?: array<string>}): Core\Test\Response`. A `?` in `$path` starts the
query. A request with none of `body`, `json`, `form` and `multipart` has no body, which is not an
empty one. `mount` is the prefix the request came in under, `""` for the root, and `captures` is the
list `Core\Request\Mount::captures()` reads back, `{1}` first. The two are one mount row, so
`captures` without a `mount`, or keyed by anything but position, is a `LogicError`: the root has no
pattern to capture.

**The peer is stated, never derived.** `clientIp` and `scheme` are what the `[server]
trusted_proxies` walk would have settled on, as a `.nvt` case's `--CLIENT_IP--` and `--SCHEME--`
are, so `Core\Request::ip()`, `isSecure()` and a `__Host-` cookie read them. No test reads
`X-Forwarded-For`. A `clientIp` that is not one IPv4 or IPv6 address, and a `scheme` that is not
`"http"` or `"https"`, is a `LogicError`. Omitted, the request has no address and is `http`. Cost:
none per request.

**`cookies` sends its pairs as one `cookie` field**, read by the same code as `--COOKIE--`. There is
no cookie jar: each call is one request, and a test copies `Response::cookies()` into the next call
itself. A `host` header given in `headers` is the request's host, and it picks the mount row the
server would pick for that host, so a link resolves through the row a real request under that host
reaches. A `traceparent` header is taken as the wire takes it, so the request continues that trace.

**A body is written with `Core\Http\Client`'s options, read by the client's own code.** `json`,
`form` and `multipart` build the body and its `Content-Type` as the client would send them, and
`contentType` names the type of a raw `body`. Only one of `body`, `json`, `form` and `multipart` may
be given, and `contentType` needs `body`. Both mistakes are compile errors, as they are on the
client, so a test writes a request the way a program writes one.

**The answer is the server's answer.** Each refusal the wire makes before the program runs is made by
the same function in a test. A forged write to a program that does not call `Core\Router::dispatch`
answers `403` with no body, and a dispatching program keeps its gate's `refused`. A body over the
run's `[limits]` answers what the wire answers for it. A `HEAD` request answers with no body, its
status and headers kept. A request whose program fails keeps the server's `500`, and
`Core\Test\Response::error(): ?string` reads the class and message of the error that ended it as
`Class: message`, `null` for a request that did not fail. It is a string, because the error object
belongs to the request's isolate and does not cross into the test's.

**The call returns once the request's after-response work has ended.** The response is the same, and
a test then reads what that work wrote. An after-response task that fails fails the test, named as
such, as a task left running does (`rule:testing/task-tree-and-virtual-clock`). On the server, that
work stays detached from the response.

**A test request gives the link to another module that `nvs serve` gives.** Where the unit links to
a route of an `autoload links` root and the configuration the test run reads writes a
`[[server.mount]]` table, the run expands that table and compiles each row's entry once, as the boot
does. The `mount` key picks the served row by its prefix, the row with no host first, and each link
is resolved by the function the server calls (`rule:routing/a-link-reaches-another-module`). With
no mount table, or a `mount` no row has, the link throws and names its handler. A tie no `canonical`
breaks leaves the link out, so it throws too, where `nvs serve` would refuse to boot.

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
program fails answers `500`, no body and no headers, which is what the server sends, unless the
program calls `Core\Router::dispatch`: then the answer is what its gate's `failed` wrote, and a request
the door refused is what its gate's `refused` wrote, exactly as the server would send them
(`rule:routing/a-gate-answers-around-every-handler`). Nothing on the
response is `tainted`: it is the program's own output.

The synthetic request's parameters arrive **`tainted`**, exactly as a real request's would, so a
handler that forgets to launder fails its test rather than production.

**The request runs on the test's clock.** A test that fixed its clock with `at:`
(`rule:testing/determinism-declared-on-the-test`) hands the request a copy of that reading, with
every `Core\Test::advance` made before the call, so an expiry the handler checks moves with the test.
Nothing the request does moves the test's clock. A `seed:` does not cross: a copy of the generator's
state would draw the same values in every request one test makes.

`#[Test(server: true)]` binds a real listener for the cases that genuinely need the wire. The port
is the operating system's and the address is loopback, so two suites on one machine never collide
and no suite serves the program under test to a network. The listener is a **sibling** of the test's
isolate rather than a child — leftover work is read off the test's own task — and it is retired when
the test that asked for it has joined. It serves under the **default** policy rather than the tree's:
reading a deployment's configuration would make the test's subject the deployment. That policy is
the server's — its headers, its CORS, its waits and its trusted proxies. The program it runs is the
tree's, so a request it answers, and every connection that request upgrades, holds the
`[capabilities]` the run's configuration names, as a request `Core\Test::request` sends does. A
served request runs on a fresh context that copies nothing from the test's, so the listener hands
each one the run's snapshot and app folder; without them an app that opens its database could not be
tested over the wire at all. **What it spends:** two reference counts per request. The two
mechanisms are both kept because they answer measurably different questions: the in-process path
never builds a response head, so the wire is not something it can be asked about.

`Core\Test::serverUrl(): ?string` reads the listener's address back, and is `null` where none was
bound, so it can be called from anywhere. A test reaching its own listener needs the outbound pair
granted — `net.connect` for the host and `net.internal` for loopback
(`rule:http-server/allow-url-pins-the-address`) — because nothing about the listener being the
test's own widens that policy.

**Declined:**
- **A cookie jar kept between `Core\Test::request` calls.** It is hidden state between two calls,
  and copying `Response::cookies()` into the next call is one line.
- **Repeated header lines.** Joining the values gives the same request, and a second `Host` line is
  a request the server refuses.
- **Server-added headers, upgrades, chunk boundaries and `Range` in an in-process request.** The
  `[http.headers]` table, CORS, the CSP nonce, a WebSocket or SSE upgrade and the framing belong to
  the wire, and `#[Test(server: true)]` serves the program on a real listener for them.
