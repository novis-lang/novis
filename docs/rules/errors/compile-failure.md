**The entry file, at request or isolate start.** No frame ever existed, so tiers 1 and 2 never had
the chance to register anything. It is reported the way any handler-less `FATAL`-class condition is:
straight to `rule:errors/handler-script`, then `rule:errors/engine-floor`. Whether an HTTP response
shows a generic page or detail is `[http.errors] detail`'s answer, not the never-started script's —
the request had no code path in which it could have decided differently. Its default follows the run
mode: generic in production, full in development.

**Mid-execution**, through `require` or a `spawn script` target failing after its parent is already
running: an ordinary `ParseError`, catchable at the call site like any `Throwable`, which is what
lets a framework fail a bad template or plugin gracefully instead of the whole request. Uncaught, it
rides the normal `rule:errors/on-uncaught-throw` path — by then a frame did exist.
