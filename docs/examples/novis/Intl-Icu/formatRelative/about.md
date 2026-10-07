Writes a list of time differences in words, such as "1 day ago" or "in 3 hours".

`Icu::formatRelative` takes a list of shapes, a locale tag and a shape of options. Each shape has a
`count` and a `unit`, which is a `TimeUnit` case. A negative count is in the past. The options are
`width` and `numeric`. The result has one string for each shape, in the same order.

`Icu::formatRelative` throws a `LogicError` when the locale tag is not valid.

**Good to know:** most programs do not call `Icu::formatRelative` directly. `RelativeTime::format` is
the same function.
