A metric label value is a sink: a query parameter, a header, a path segment or a database column
cannot become a label.

There is deliberately **no launderer for it**, because there is no sanitisation that would make one
safe. The hazard is not the value's content, it is that the value is drawn from an unbounded set, and
an unbounded label set is how a metrics collector falls over. What exists instead is everything that
is already unqualified and is what a label should have been: an enum case or an integer converted with
`as`, which laundering makes free (`rule:security/taint-propagation`); a route's path pattern from the
compiled table; a literal, a class constant or a configured value; and, for a genuinely bounded user-derived
set, `rule:security/assert-trusted`.

`secret` is refused there too and needs no new rule: a metric export is output, and every output sink
already refuses it (`rule:security/secret-sinks-refuse`).

The cost is a compile error at exactly the line that would have taken the collector down, and it is
there because the qualifier system already exists for injection. Labelling by a user id, a tenant
name or an error message is what people reach for first, so this refusal is met often.

**Declined:**
- **Allowing `tainted` labels behind a runtime cardinality guard.** The guard fires after the
  collector already has the series, and a compile error at the responsible line is strictly better
  than a warning hours later on another machine.
