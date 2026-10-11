`nvs.toml` defines what a request **starts with**. A directive is a limit that cannot be exceeded
only when it cannot be changed at runtime at all; where a value can change, the request sets whatever
it wants, wider or narrower, and the file's value is the starting point it inherits.

Every directive carries one of three changeability classes in the registry:

| Class | `nvs.toml` | `Core\Config::set` |
|---|---|---|
| `System` | the only place it can be set | fails, returns `false` |
| `Runtime` | the **default** a request starts with | any value, wider or narrower, up to the ceiling |
| `RuntimeTighten` | the default *and* an upper bound | narrowing only; widening fails |

`Runtime` is the class for anything changeable. `RuntimeTighten` is argued per directive, never as a
policy: it exists for capability grants, where a script may drop a right it holds and never add one it
does not, and for directives that only ever narrow, such as `open_basedir`. The class answers
one question only — *who may set it*. What applying a change requires is a second field on the same
entry (`rule:config/reloadability-is-its-own-field`).

This is what lets the most common run-time set — raising `memory_limit` for one import — work, while
`rule:config/ceilings-are-their-own-directives` keeps one request from becoming every co-resident
request's outage. One directive cannot answer both questions — what a script gets by default, and
what the host tolerates from one request — and keeping them apart is what spares an operator from
raising the global limit for everyone to let one import through. No runtime set crosses an isolation
boundary (`rule:config/a-runtime-set-is-request-local`), so **Security and request isolation** is not
what widening trades.

**Declined:**
- **Narrowing only, for every limit.** The commonest run-time set raises a limit, so code that does it
  would fail at run time with the out-of-memory error or timeout it was trying to prevent.
- **A hardened mode that makes every limit narrowing-only again.** Two configuration semantics to
  implement, test and reason about, for what one ceiling per directive already says.
