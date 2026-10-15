`propertyPath<T>` is a type atom beside `property<T>`, written in every position a type is written,
whose **values** are dotted chains of public declared property names starting at `T`: `customer`,
`customer.address.city`, `lines.product.name`. What is carried at run time is the dotted text, a string
in the representation but **not** in the type.

Every segment but the last names a property whose declared type is a class `C`, a `?C`, or an array of
`C` (`array<C>`), and the next segment is looked up on `C`. The last segment is any public
property. A segment that names nothing, names a `private` or `protected` property, or steps through a
property whose type is not one of those three forms is the same failure, and fails the same way at
every segment. `T` follows `rule:types/property-key` exactly: one class, `E0799` for anything else or
for a class with no public property; visibility decided at the conversion, once; the argument
contravariant (`rule:types/property-key-variance`); and its own equality domain, two paths equal when
their text is equal, with ordering refused. `propertyPath` is a keyword only in type position, as
`property` is.

**`as` is its source, and a literal at a key parameter is the other** (`rule:types/a-literal-at-a-key-parameter-is-checked-where-it-is-written`).
A `string` converts with `as propertyPath<T>`, which throws at the first bad segment and names it, and
`as ?propertyPath<T>` yields `null` there. A written-out operand is decided where it is written: a
wrong segment is the unknown-member diagnostic an ordinary `$order->custmer` gets, placed on that
segment. `propertyPath<T> → string` is total and gives the dotted text. The conversion strips `tainted` for
the reason `rule:types/property-key` strips it: its whole output range is chains of names the program
declares `public`.

The run-time conversion costs one field lookup and one visibility test per segment, the pair the key
conversion performs once, and has **no length limit**: a path that names the same relation many times
over is checked one segment at a time, so its cost is linear in what the caller sent and nothing is
held per segment beyond the walk. A path that loops through a class back to itself is legal and costs
the same, because each step reads one class's roster.

The type exists because a field named across a relation — a sort column `customer.name`, a filter
`lines.product.sku`, a form field for a nested object — is otherwise a string the compiler cannot
check, or a chain of `property<T>` keys a library has to join by hand. The roster each step reads is
the one `property<T>` already records per class (`rule:classes/no-dynamic-properties`), so a path adds
no second walk.

- **Memory:** a path is one string value, its dotted text. The checker keeps one roster per class a
  path reaches, as `property<T>` does, for the length of the analysis. Nothing is spent per request
  beyond the string, and the run-time conversion allocates nothing past it.

**Declined:**
- **Reading or writing through a path, `$obj->{$path}`, for now.** A path through an array would read
  an array of values, and what that read returns, and what a write through it means, is a decision of
  its own. A library walks the path with `Core\Reflect::steps`
  (`rule:core-classes/str-parts-and-reflect-steps`) until that is decided.
- **A path as an array of keys, `array<property<T>>`.** Each key would be typed against a different
  class, which no single element type expresses, and a request carries a sort field as one string.
- **A depth limit.** The cost is already linear in the text the caller sent, and the request's own
  limits bound that text. A limit would be a second number to configure with nothing it protects.
