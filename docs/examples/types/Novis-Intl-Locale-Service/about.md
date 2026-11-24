The kind of data that `Locale::resolve` checks, such as the data for sorting text or writing dates.

`Locale::Service` is declared in the class `Novis\Intl\Locale`.

`Collation` is the data for sorting text. `Segmentation` is the data for splitting text into words.
`Numbers`, `Plurals`, `Dates` and `Lists` are the data for writing numbers, plural forms, dates and
lists. `RelativeTime` is the data for times before or after now, such as "in 3 days".

A locale tag is a short name for a language and a region. For example, `de-AT` is German for
Austria. You pass a list of tags to `Locale::resolve`. For each tag, it returns the locale whose data
is used for the kind you name.

When a tag has no data of its own, a more general locale is used. For `de-AT`, this can be `de`,
which is German. The result `und` means that no language has this data for the tag. Then the base
rules are used. Every locale starts from these base rules.

The result can be different for each kind. `de-AT` has its own rules for numbers. For dates, it uses
the German data, so the result is `de`.

**Good to know:** use this to tell a user which language your page is really shown in.
