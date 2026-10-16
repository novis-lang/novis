`Core\Test::serverUrl()` returns the address of a real web server for one test. A test gets this
server when it is declared with `#[Test(server: true)]`. The server runs on your own computer, on a
free port that the operating system chooses. The address looks like `http://127.0.0.1:52341`. It has
no `/` at the end, so you can add a path to it directly.

The server answers each request by running your program, the same way a real server does. Use it
when a test needs a real network connection, for example to test an HTTP client. For most tests,
`Core\Test::request` is simpler and faster.

Everywhere else, `serverUrl()` returns `null`. This includes a test without `server: true` and a
program started with `nvs run`.

**The examples below** show the `null` result outside a server test, a test that calls its own
server, and a test of a small health check client.
