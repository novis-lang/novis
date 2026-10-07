Every `Novis\Intl` member takes its locale as a BCP 47 tag argument. There is no default locale, no
setting that holds one and no `[intl]` configuration block (`rule:core-api/no-ambient-state`).

- **A malformed tag** throws `LogicError` naming it, cut to its first 64 characters. A locale is the
  program's own data, so a bad one is a bug.
- **A well-formed tag with no data** falls back along CLDR's chain — `de-AT` to `de`, and at last to
  the root locale — and formats. It never throws.
- **`Locale::resolve`** returns, for each tag and a `Service`, the locale whose data answers: `de` for
  `de-XX`, `und` where only the root does. Coverage differs between services, so the service is an
  argument.
- **A `-u-` keyword that an option also names** — `ks`, `kf` or `kn` beside `strength`, `caseFirst`
  or `numeric` — throws `LogicError`, so no option has two spellings. A keyword no option names, such
  as `co`, `nu`, `ca` or `hc`, is read from the tag.

`Locale::negotiate` never throws for its header: an empty, malformed or unmatched `Accept-Language`
returns the default, and only its first 32 ranges are read. A malformed tag among the offered locales
or the default throws `LogicError`, because those are the program's.
