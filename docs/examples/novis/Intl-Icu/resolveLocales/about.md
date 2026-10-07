Shows which locale's data is used for each locale tag in a list.

`Icu::resolveLocales` takes a list of locale tags and a `Service` case. It returns one tag for each,
in the same order: the locale whose rules are used for that service. With `Service::Dates`, `"de-AT"`
gives `"de"`. The result is `"und"` when only the general rules are used.

`Icu::resolveLocales` throws a `LogicError` when a tag is not valid.

**Good to know:** most programs do not call `Icu::resolveLocales` directly. `Locale::resolve` is the
same function.
