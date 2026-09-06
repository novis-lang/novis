The `route` label on the request series carries the route's **declared name** from the compile-time
table of `rule:routing/routes-are-compiled-not-registered` — a closed set known when the program is
built — and never the request's raw path. A program with no route table simply has no `route` label,
and a matched route that declared no name has none either; there is no fallback to the path, because
`/users/1`, `/users/2`, … is one series per user, which is the cardinality bomb every label rule in
this chapter exists to prevent, and offering it as an option means it would be chosen.

This is the one place the route table is read by something other than the application, and the
label reads the match the request already made rather than matching again. Per-endpoint latency is
the second thing anyone looks at on a dashboard, so an application that does not use the route
table loses something real; the alternative was worse, and the coupling is deliberate.

A route name is one of the values that are already unqualified and are what a label should have
been, which is why it passes `rule:security/metric-label-refuses-tainted` for free.
