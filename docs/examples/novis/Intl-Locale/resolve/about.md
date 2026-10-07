Shows which locale's data is used for a locale tag.

`Locale::resolve` takes a list of locale tags and a `Service`, and returns one tag for each, in the
same order. The result is the locale whose rules the `Novis\Intl` classes use for that service. When
there is no data for a region, the language's data is used. With `Service::Dates`, `"de-AT"` gives
`"de"`.

The result is `"und"` when only the general rules are used. That happens for a tag with no data at
all, such as `"zz"`. It also happens when a language has no special rules for that service. German
sorts with the general rules, so `"de"` with `Service::Collation` gives `"und"`.

`Service` has one case for each class: `Collation`, `Numbers`, `Plurals`, `Dates`, `RelativeTime`,
`Lists` and `Segmentation`.

`Locale::resolve` throws a `LogicError` when a tag is not valid.

**Good to know:** the other `Novis\Intl` methods do not throw for a valid tag with no data. They use
the general rules. Call `Locale::resolve` when you need to know which data was used.
