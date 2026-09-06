No variable is ever populated by the host. Every fact PHP hands a script through a superglobal is returned
by a `static` method on a reserved `Core` class, populated per isolate:

| PHP | replacement |
|---|---|
| `$_SERVER` | `Core\Server` — server metadata and request headers |
| `$_GET`, `$_POST`, `$_COOKIE`, `$_FILES` | `Core\Request` — query, body, cookie and upload input |
| `$_SESSION` | `Core\Session`, after an explicit `Core\Session::start()`; there is no auto-start |
| `$_ENV`, `getenv()` | `Core\Env` |
| `$argv`, `$argc` | `Core\Cli` — the CLI entry point only |
| `$_ARGS` | `Core\Script::args()` |

`Core\Server` and `Core\Request` are two classes rather than one, matching the split between facts about
the server and input the client sent. The value handed back keeps the shape any other boundary value has —
structured input as `array<mixed>`, a scalar payload not yet asserted to be text as `bytes` — so it needs
an explicit `as` before it populates a typed binding.

Each rejected spelling is diagnosed by name and names the `Core` class that replaces it.
