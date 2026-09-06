A metric label value is a sink: a query parameter, a header, a path segment or a database column
cannot become a label.

There is deliberately **no launderer for it**, because there is no sanitisation that would make one
safe. The hazard is not the value's content, it is that the value is drawn from an unbounded set, and
an unbounded label set is how a metrics collector falls over. What exists instead is everything that
is already unqualified and is what a label should have been: an enum case or an integer converted with
`as`, which laundering makes free (`rule:security/taint-propagation`); a route name from the compiled
table; a literal, a class constant or a configured value; and, for a genuinely bounded user-derived
set, `rule:security/assert-trusted`.

`secret` is refused there too and needs no new rule: a metric export is output, and every output sink
already refuses it (`rule:security/secret-sinks-refuse`).

The `Core\Metrics` surface this governs is not on disk: nothing in the tree exports a series or takes
a label, so the refusal has no implementation to verify against.
