The viewer renders the correlated request stream and the log, and every artifact that an existing
tool already renders is handed to that tool rather than drawn.

A profile is handed to speedscope in the format `rule:observability/speedscope-timeline-export`
already commits to, and a coverage report to whatever reads Clover or lcov. A coverage **heatmap**
over source is drawn, because that is a per-line background over text rather than a viewer. This is
`rule:ide/the-extension-builds-no-ui-the-editor-already-has`'s test applied rather than set aside:
where a renderer exists, feed it. The live, correlated request stream is the one thing here that has
no incumbent in any ecosystem, which is the whole of the exception.

The boundary is written as a rule rather than left as an intention because the failure mode of an
in-house dashboard is that it slowly grows a second copy of every tool the project deliberately did
not build.

**The request is the primary object.** The main view lists requests — route, status, duration, query
count, dump count — and one opens to its own timeline; the flat filterable stream is the second view,
not the front page. The correlation key is the trace id every request already carries
(`rule:observability/a-trace-id-exists-for-every-request`), so it costs nothing to derive. In
development the response also carries that id in a header so the viewer can move from the call being
looked at to its timeline; in production it does not, being a correlation and fingerprinting leak.

The value tree the browser receives is already finite — `rule:errors/record-transformations` bounded
it with elision and resolved every repeat to an identity before it was written — so a repeat renders
as a link to the node it names and expansion needs no fetch. Children are built from data already
loaded.

**The listener is the service's own.** The viewer is a subcommand, binds loopback by default, and is
never mounted on the application's listener: there every path-normalization bug and proxy
misconfiguration would become disclosure, and a reserved path prefix would collide with the
compile-time route table. A non-loopback bind needs TLS and a token read from a file, and boot fails
if the file is absent. Because it is a browser-facing local listener, `Host` and `Origin` are
allowlisted against DNS rebinding, its renderer builds DOM rather than assigning markup, and it runs
no Novis code.

The viewer's assets ship in every binary. What keeps that cheap is reachability rather than size: a
server process never has a viewer in it.

**Declined:**
- **Fetching a value tree lazily as the developer expands it.** The tree is already bounded before it
  is written, so a fetch per expansion would add round trips and server-side state to make a bounded
  thing safe again.
