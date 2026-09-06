There is **no customization hook**: no `DebugRepresentable`, no `__debugInfo`, no per-class
renderer. `rule:errors/diagnostic-record` being a closed model is that refusal expressed as a data
type. A dump shows a class's real declared properties and their real current values, with
`rule:errors/record-transformations` and nothing else.

**A dump does not call `Stringable`.** A `toString` result would be a second, prettier, possibly
lying view of the state a dump exists to show.

Adding a fourth rendering later is a change to one crate, not to any call site — which is the
property that makes refusing one now cheap to revisit.
