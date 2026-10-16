Returns the kind of run your code is in, as a `Core\Env::Context` case. It never throws an error.

| Case | Your code runs |
|---|---|
| `Request` | to answer an HTTP request, or a request made with `Core\Test::request` |
| `Command` | from the command line: a script, or a `#[Command]` method |
| `Isolate` | in a child script started with `spawn script` |
| `Schedule` | as a scheduled script from a `[[schedule]]` section |
| `Job` | as one attempt of a queued job |
| `Test` | in a `#[Test]` method under `nvs test` |
| `Connection` | for a WebSocket or event stream connection after `upgrade` |

A task started with `Core\Task` has the same kind of run as the code that started it. A child
script is `Isolate`, even when a request started it. A request is always `Request`. No header,
environment variable or setting can change that.

Use it when one file does different work in different places. For example, a file can serve a page
in a request and test that page when you run it from the command line.
