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

A superglobal is untrusted input with no declared boundary: it is simply present, readable and writable
from any scope. A method call on a named class is a declared, traceable entry point, so a reviewer sees
which class, and therefore which trust boundary, each line depends on. With this rule a script learns
anything from outside itself in one way only — a declared class member, an object property or a
parameter — the same answer `rule:statements/storage-that-outlives-a-call` gives for state.

**Declined:**
- **The superglobals kept as read-only variables.** A read-only bare variable is still not imported
  or declared anywhere, and it fits no row of `rule:statements/storage-that-outlives-a-call`.
- **One class for every request and server fact.** One class for unrelated concerns is a global
  namespace with extra syntax (`rule:classes/no-free-functions-or-constants`).
