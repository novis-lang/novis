Anything with an external specification — a protocol, a parser, a wire format, a cipher, a codec, a
cron expression, a timezone database — is implemented by an existing first-class Rust crate. Anything
about Novis's own compiler or scheduler has no possible crate and is written here. This is the
question beside `rule:packaging/a-c-dependency-answers-two-questions`: not what the tree may depend
on, but what it may write itself.

A second-rate implementation of somebody else's specification is a security surface the project would
then own for as long as the feature exists, so the default is that a feature without such a crate is
not built. Writing one anyway is the exception, argued in the feature's own rule against that cost:
`rule:core-classes/db-crate-boundary` does it for TDS, the one database wire with no crate, because
SQL Server is in the driver matrix at all only that way.

The split is the one the tree already lives with. Cranelift compiles, and the lowering into it is
written here; `serde_json` parses, and the derive that emits Novis IR from Novis types
(`rule:core-classes/derive-attribute`) could not be a crate. When a feature is half of each — a
database driver borrows its codec and writes its state machine, an exporter borrows its encoder and
writes its wiring (`rule:observability/the-exporters-are-crates`) — which half is which is said
before either is written.
