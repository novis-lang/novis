The kind of language data that `Locale::resolve` looks up, such as the data for sorting text.

`Locale::Service` is declared in the class `Novis\Intl\Locale`.

`Collation` is the data for sorting text. `Segmentation` is the data for splitting text into words.
`Numbers`, `Plurals`, `Dates` and `Lists` are the data for writing numbers, plural forms, dates and
lists. `RelativeTime` is the data for times such as "in 3 days".

A locale is a language and a region. Its tag is a short name, such as `de-AT` for German in Austria.
You pass tags and one `Locale::Service` case to `Locale::resolve`. For each tag, it returns the
locale whose data is used for that case.

When a tag has no data of its own, a more general locale is used. For `de-AT`, this can be `de`. The
result `und` (undetermined) means that no language has this data. Then the base rules are used.

The result can be different for each kind. `de-AT` has its own rules for numbers, and it uses the
German data for dates.

**Good to know:** use this to tell a user which language your page is really shown in.
