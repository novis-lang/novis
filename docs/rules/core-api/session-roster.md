The session surface is twelve static members, and every write states the lifetime of what it writes
(`rule:http-server/a-session-value-is-short-lived-or-long-lived`):

| Member | What it does |
|---|---|
| `setShortLived($key, $value)` | Saves a value with the short lifetime |
| `setLongLived($key, $value)` | Saves a value with the long lifetime |
| `get($key)` | Reads a value of either lifetime |
| `remove($key)` | Deletes one value of either lifetime |
| `clearShortLived()` | Deletes every short-lived value, and moves the session to a new identifier |
| `clearLongLived()` | Deletes every long-lived value, and moves the session to a new identifier |
| `clearAll()` | Deletes the session, and expires both cookies |
| `regenerate()` | Moves the session to a new identifier and keeps every value |
| `setShortLivedSecret($key, $value, $keys)` | Saves a sealed secret with the short lifetime |
| `setLongLivedSecret($key, $value, $keys)` | Saves a sealed secret with the long lifetime |
| `getSecret($key, $keys)` | Opens a sealed secret of either lifetime |
| `openWithIdentifier($id)` | Opens the session under an identifier the program read itself |

**There is no plain `set`.** A write that does not say how long it lives would pick one lifetime for
the program, and the wrong one is either a login that survives a browser restart or a consent choice
lost with the browser. All values share one set of keys, and the last write of a key sets its
lifetime, a sealed secret's too.

**The first call of any member opens the session** (`rule:core-classes/a-session-opens-on-its-first-use`).
There is no `start`; `openWithIdentifier` is the one member that opens a session explicitly, and it
throws once the session is open. The sealed members follow the same open before they look at a key
ring (`rule:http-server/a-session-holds-a-secret-only-sealed`).

`regenerate` moves the stored session to a new identifier in one step, so the old identifier stops
working at once and a key another request wrote survives the move. It takes no argument: a flag to keep
the old entry would choose between a fixation window and a lost session, and only one of those is
correct. `clearShortLived` and `clearLongLived` move the session the same way, so each clear is also a
fixation defence.

**Every member that writes throws where the head is closed**, inside `Core\Html::later` and
`Core\Task::afterResponse`, and the checker refuses the call where it is written there
(`rule:concurrency/deferred-work-cannot-write-the-response`). `get` and `getSecret` compile in
both places. Inside `Core\Html::later` they read the request's session, and after the response they
throw `LogicError`, because deferred work has no session.

**Declined:**
- **`start`, `set`, `clear`, `destroy` and `setSecret` kept as aliases.** Novis is pre-alpha, and two
  ways to write one thing is a surface every reader learns twice.
- **A plain `set` with a default lifetime.** Every write states its lifetime, so a reader of the code
  never has to look up which one a default picked.
