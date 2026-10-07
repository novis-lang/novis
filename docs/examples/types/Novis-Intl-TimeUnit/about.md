The unit of an amount of time that `RelativeTime::format` writes, such as "5 minutes ago".

Each item you pass to `RelativeTime::format` has a `count` and a `unit`. The unit is one of
`Second`, `Minute`, `Hour`, `Day`, `Week`, `Month`, `Quarter` and `Year`. A negative count is in the
past and a positive count is in the future, so `{count: -2, unit: TimeUnit::Week}` is "2 weeks ago".

The words and the plural form come from the locale.

**Good to know:** `RelativeTime::format` does not choose the unit for you. Your program decides
whether 90 minutes is "1 hour ago" or "90 minutes ago".
