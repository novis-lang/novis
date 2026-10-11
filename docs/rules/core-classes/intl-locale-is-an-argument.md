Every `Novis\Intl` member takes its locale as a BCP 47 tag argument. There is no default locale, no
setting that holds one and no `[intl]` configuration block (`rule:core-api/no-ambient-state`).

- **A malformed tag** throws `LogicError` naming it, cut to its first 64 characters. A locale is the
  program's own data, so a bad one is a bug.
- **A well-formed tag with no data** falls back along CLDR's chain — `de-AT` to `de`, and at last to
  the root locale — and formats. It never throws: a tag a program got from a user or a header is
  well-formed and unknown far more often than it is wrong.
- **`Locale::resolve`** returns, for each tag and a `Service`, the locale whose data answers: `de` for
  `de-XX`, `und` where only the root does. Coverage differs between services, so the service is an
  argument. It is how a program learns which locale answered.
- **A `-u-` keyword that an option also names** — `ks`, `kf` or `kn` beside `strength`, `caseFirst`
  or `numeric` — throws `LogicError`, so no option has two spellings. A keyword no option names, such
  as `co`, `nu`, `ca` or `hc`, is read from the tag, so a user's preference travels with their locale.

`Locale::negotiate` never throws for its header: an empty, malformed or unmatched `Accept-Language`
returns the default, because the header is request data the program does not control. It matches each
range against the offered tags with CLDR's fallback and returns the offered tag as the program spells
it. Only the first 32 ranges are read, which keeps a hostile header's cost fixed; no browser sends a
tenth of that. A malformed tag among the offered locales or the default throws `LogicError`, because
those are the program's.

**Declined:**

- **A default locale** — a setting, a per-request value, or the current request's `Accept-Language`
  read implicitly. Each is ambient state.
- **Throwing for an unknown but well-formed locale.** A page that throws for `gsw` is worse than one in
  German.
