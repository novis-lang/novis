The set of HTTP verbs a route can be declared under, and a request can arrive with. There are eight
of them and there is no ninth.

`Get`, `Head`, `Options` and `Trace` only read something. `Post`, `Put`, `Patch` and `Delete` may
change it, and those four are the ones the cross-site request check covers. You name a case when you
declare a route, and you get cases back when you ask the router which verbs claim a path.

Because the set is closed, a verb is a value the compiler knows rather than text you spell out. A
misspelled verb is refused while you write the program, instead of becoming a route nobody can
reach.

**Good to know:** `CONNECT` is missing on purpose. It asks a proxy to open a tunnel, which is not
something a route of yours answers.
