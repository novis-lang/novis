`Core\Config::set` writes into the request's own copy-on-write overlay over the published snapshot,
and the overlay is discarded when the request ends. `Core\Config::get` reads the effective value —
the overlay, then the snapshot — and `Core\Config::restore` drops only what this request set,
returning a directive to the file's value.

A widened limit is therefore never observable to another request and cannot outlive the one that
set it, which is what makes widening a question about *this* request's share of the host rather
than about isolation. No runtime set can grant a capability, load an extension, reach another
request's heap, or exceed a `System` value; what a request can do is move its own budget within the
range the operator fixed, in both directions.

The same property is what makes a request-local mode flip safe at all
(`rule:config/a-program-may-read-and-flip-its-mode`), and it is the per-request half of the split
`rule:config/the-config-is-an-immutable-snapshot` completes: requests share nothing *mutable*, and an
overlay each request owns is not shared.
