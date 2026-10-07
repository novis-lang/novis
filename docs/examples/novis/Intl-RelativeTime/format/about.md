Writes an amount of time before or after now, such as "3 days ago" or "in 2 hours".

`RelativeTime::format` returns one string for each item, in the same order as the list. Each item has
a `count` and a `unit`, such as `{count: -3, unit: TimeUnit::Day}`. A negative count is in the past,
so that item is "3 days ago". A positive count is in the future, so `{count: 2, unit: TimeUnit::Hour}`
is "in 2 hours". The locale decides the words. In German, the first item is "vor 3 Tagen".

The options change how the text looks. `numeric: Numeric::Auto` uses a word where the language has
one, such as "yesterday" for a count of `-1` days. The default is `Numeric::Always`, which writes "1
day ago". `width: Width::Short` writes "in 2 hr.", and `Width::Narrow` is shorter still where the
language has a shorter form.

`RelativeTime::format` throws a `LogicError` when the locale tag is not valid.

**Good to know:** this method does not read the clock. You work out the count and the unit yourself,
for example from the difference between two `Core\Time` values.
