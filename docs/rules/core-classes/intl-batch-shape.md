Every `Novis\Intl` member except `Locale::negotiate` takes a list and returns a list of the same length
in the same order, and makes exactly one host-to-guest call. A single value is a list of one, at the
same one call, so sorting 10,000 names or formatting a page of prices costs one crossing and one copy
each way (`rule:packaging/the-boundary-is-the-cost`).

One call has one locale. A program whose rows carry different locales makes one call per locale.

`negotiate` is the one exception, because its input is one `Accept-Language` header from one request.

The interface holds the shape: every export of `nvs:intl/icu` but `negotiate` takes a `list` as its
first parameter and returns a `list`, and `crates/nvs-stdlib/tests/ext_world.rs` fails naming an export
that does not. `Collator::sort`'s export returns the strings' positions, so only numbers cross back.
