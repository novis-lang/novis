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
reading a deployment's configuration would make the test's subject the deployment. The two
mechanisms are both kept because they answer measurably different questions: the in-process path
never builds a response head, so the wire is not something it can be asked about.

`Core\Test::serverUrl(): ?string` reads the listener's address back, and is `null` where none was
bound, so it can be called from anywhere. A test reaching its own listener needs the outbound pair
granted — `net.connect` for the host and `net.internal` for loopback
(`rule:http-server/allow-url-pins-the-address`) — because nothing about the listener being the
test's own widens that policy.
