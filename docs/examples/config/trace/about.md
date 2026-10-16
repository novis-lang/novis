Whether this deployment records what a request did, and how much of it.

A trace is the story of one request across every process that handled it. This block says where those
spans are shipped, what fraction of the requests this process *starts* are recorded, and whether a
call it makes outbound carries the trace it belongs to, so the service on the other end can add its
own part of the story.

**In plain words:** the exporter says who is told, and the sample says how often. Writing only the
first one gets you a collector nothing arrives at, because the fraction starts at none.

The whole block is the operator's. A program that could raise its own sample would be selecting the
requests worth recording, and one that could lower it would be choosing not to be looked at — which
is exactly the kind of decision a trace exists to make visible. A developer who wants a trace of
their own request has `Core\Debug`, and nothing here affects it.

The example prints what this checkout records and is turned away trying to change each of the three
things about it that matter.
