Joins the items of each list into one string, with the words a language uses.

`Icu::formatLists` takes a list of lists of strings, a locale tag and a shape of options. The options
are `type` and `width`. It returns one string for each list, in the same order. In German,
`["Shop", "Blog", "Wiki"]` gives "Shop, Blog und Wiki".

`Icu::formatLists` throws a `LogicError` when the locale tag is not valid.

**Good to know:** most programs do not call `Icu::formatLists` directly. `ListFormat::join` is the
same function.
