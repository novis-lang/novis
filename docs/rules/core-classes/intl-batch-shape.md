Every `Novis\Intl` member except `Locale::negotiate` takes a list and returns a list of the same length
in the same order, and makes exactly one host-to-guest call. A single value is a list of one, at the
same one call, so sorting 10,000 names or formatting a page of prices costs one crossing and one copy
each way (`rule:packaging/the-boundary-is-the-cost`).

One call has one locale. A program whose rows carry different locales makes one call per locale, which
is the shape it has anyway when it groups output by reader. A per-item locale would put a string per
item into every crossing, for a case no web page has.

`negotiate` is the one exception, because its input is one `Accept-Language` header from one request,
and a batch of headers is not a thing a request holds.

The interface holds the shape: every export of `nvs:intl/icu` but `negotiate` takes a `list` as its
first parameter and returns a `list`, and `crates/nvs-stdlib/tests/ext_world.rs` fails naming an export
that does not. `Collator::sort`'s export returns the strings' positions, so only numbers cross back and
the Novis half reorders the list it already has; equal strings keep their input order.

- **Memory:** a batch costs its input and its output, copied once each way. A large batch is bounded by
  the request's memory cap like any other value.

**Declined:**

- **A comparator for collation.** A sort of 10,000 strings would make about 130,000 crossings.
- **A per-value member beside each batch member.** It is a second way to reach every operation, and
  saves only a pair of brackets.
