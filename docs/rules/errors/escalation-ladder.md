Nothing Novis runs is silently dropped, but not everything is *caught* — those are different
guarantees. A `FATAL`, or a `THROWN` that reached the request root uncaught, escalates through up
to four tiers, each with a fixed budget, before falling to the next:

| Tier | What runs | Rule |
|---|---|---|
| 1 | the request's own limit handler | `rule:errors/on-limit` |
| 2 | the request's own uncaught-throw handler | `rule:errors/on-uncaught-throw` |
| 3 | the operator's configured `.nvs` handler | `rule:errors/handler-script` |
| 4 | the hardcoded engine floor | `rule:errors/engine-floor` |

**No tier is retried.** A handler that throws, panics, or exceeds its own budget is abandoned where
it stands and the failure drops to the next tier — never to the same one again. A second attempt is
how "catch and log" becomes an infinite loop, so there is no bounded-N knob to size either.

Every tier that writes a log line writes the same record through the same native serialiser
(`rule:errors/log-write`), so a dashboard never reconciles two shapes depending on which tier
produced a line.

**Declined:**
- **A `catch (Throwable)` that intercepts a `FATAL` anywhere in the stack.** It would reopen the
  status model of `rule:errors/propagation` and spread the catch-loop risk to every stack depth
  instead of containing it at one boundary (`rule:errors/throwable-hierarchy`).
- **An engine-only logger with no script tier.** Routing and formatting errors would move to a
  second, host-side configuration surface instead of the language the application is written in.
