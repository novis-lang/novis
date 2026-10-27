The `route` label on the request series carries the matched route's **path pattern**, the
mount-relative `#[Route]` `path` such as `/users/{id}`, from the compile-time table of
`rule:routing/routes-are-compiled-not-registered` — a closed set known when the program is built — and
never the request's raw path. Every matched route has the label. A program with no route table, and a
request no route matched, has no `route` label, and there is no fallback to the raw path.

The verb stays in its own label where the series has one, and the `route` label adds none, so a method
serving several verbs shares one pattern and the series splits by verb only where it already did.

This is the one place the route table is read by something other than the application, and the
label reads the match the request already made rather than matching again. Per-endpoint latency is
the second thing anyone looks at on a dashboard, so an application that does not use the route
table loses something real; the alternative was worse, and the coupling is deliberate.

A path pattern is one of the values that are already unqualified and are what a label should have
been: it comes from the compiled table and never from the request, which is why it passes
`rule:security/metric-label-refuses-tainted` for free. It is also what a reader of the dashboard
recognises, and a rename of the handler method keeps its series.

**Declined:**
- **Labelling by the request path**, which would need no route table. `/users/1`, `/users/2`, … is
  one series per user, the cardinality bomb every label rule in this chapter exists to prevent, and
  offering it as an option means it would be chosen.
- **Labelling by the handler's `Class::method`.** A refactor that renames a method would break the
  dashboard's series, and the name means nothing to an operator who did not write the code.
