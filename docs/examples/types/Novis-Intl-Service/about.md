The kind of data `Locale::resolve` checks: the rules for sorting, numbers, plurals, dates, relative
times, lists or splitting text.

The cases are `Collation`, `Numbers`, `Plurals`, `Dates`, `RelativeTime`, `Lists` and
`Segmentation`. For each tag you pass, `Locale::resolve` returns the locale whose data is used for
that kind. When a tag has no data of its own, a more general locale is used. "und" means the root
rules, which every locale falls back to.

The answer can be different for each kind. German for Austria has its own number rules, but uses the
German dates.

**Good to know:** use this to tell a user which language your page is really shown in.
