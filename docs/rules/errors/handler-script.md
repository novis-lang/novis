`[log] handler = 'path/to/handler.nvs'` names a script invoked as an ordinary `spawn script`
isolate — fresh arena, fresh globals, its own config overlay, sharing nothing but compiled code —
receiving one explicit report argument through `Core\Script::args()`.

It cannot use ambient `Core\Request`, `Core\Server` or `Core\Session`, and structurally could not
be trusted to: for a compile failure no request context exists yet, and for a panic the runtime's
own state is what is in question. That restriction is not new here; it already applies inside any
spawned isolate.

**The one deliberate exception to how an isolate is funded.** An ordinary child spends its parent's
budget, which is exactly wrong here — a request already at its ceiling has nothing left to give,
and still running when that is true is the whole point of this tier. So this isolate is charged to
a small, fixed, **engine-owned** allotment sized once per worker at boot rather than per request.
It is a named carve-out for this purpose and not a precedent.

The tier is the shared catch-all: it fires for any tier-1 or tier-2 failure, for a handler that was
never registered at all, for an internal panic unconditionally, and for an entry-script compile
failure where no frame ever existed to register anything from.
