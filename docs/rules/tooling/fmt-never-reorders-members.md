Methods, properties, class constants and enum cases keep the order their author wrote. No "group by
visibility, constants before properties before methods" rule exists, because declaration order is
observable: `rule:core-classes/derive-field-list` makes a derived codec's encode order the property
declaration order, on purpose, for ETags and cached fixtures; and `rule:testing/bench-counters` makes the
test runner's report order the declaration order of the cases. A formatter that reordered members would
change what a program prints and sends, and a formatter that changes meaning is not a formatter — the
same line `rule:core-api/written-visibility` takes when it refuses to let the formatter insert a missing
`public`.

A developer who wants the reordering can have it as a deliberate, diff-visible code action. It
is never something a formatter does on save. The `use` block (`rule:tooling/fmt-sorts-the-use-block`) is
the only reordering anywhere.
