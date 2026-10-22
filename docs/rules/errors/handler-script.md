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

**Server work does not wait for the handler.** A request, a connection, a scheduled run or a job
that fails starts the handler on a task of its own and goes on, so its response is written and its
in-flight place goes back while the handler runs. A client who can make a program throw would
otherwise hold a place, and the core's CPU, for up to `handler_reserve_time` per request. One core
runs at most four handlers at once; past that the run is dropped, counted in
`nvs_log_handler_dropped_total`, and the floor writes the record instead. A handler that fails on
its own task has the floor write the record there, so each failure is still reported once. Its
output goes to the diagnostic stream, because no response is waiting for it. A CLI program still
waits for its handler, because the process ends with the program.

**Memory:** the allotment is a ceiling, 16 MiB by default, and the same whatever the failing
request was allowed. Nothing is held between failures; in flight the handler costs one more isolate
context, and off a request one more context for its task, at most four per core, so it does not
grow with the number of requests served.

**Declined:**
- **Queueing the handlers past four for a later turn.** A queue is a second store that grows with
  how far behind the handlers are, and the floor already writes the record, with repeats counted.
- **Joining the handler on the failing request.** It keeps the handler's output in that request's
  stream, which a failed response does not send, and it costs a place per forced failure.
