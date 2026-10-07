The intl component's classes are `Collator`, `NumberFormat`, `PluralRules`, `DateFormat`,
`RelativeTime`, `ListFormat`, `Segmenter` and `Locale`, all under `Novis\Intl` and written in Novis
source over one manifest class, `Novis\Intl\Icu`, whose static methods are the exports of
`nvs:intl/icu` in `wit/intl.wit`.

| Class | Members |
|---|---|
| `Collator` | `sort`, `sortKeys` — the strings in the locale's order, or one byte key per string that orders the same way |
| `NumberFormat` | `decimal`, `percent`, `currency`, `compact` |
| `PluralRules` | `cardinal`, `ordinal` — each returns `Core\Cldr\PluralCategory` |
| `DateFormat` | `dateTimes`, `dates`, `times` — over `Core\Time\DateTime`, `Date` and `TimeOfDay` |
| `RelativeTime` | `format` — a count and a `TimeUnit`, negative in the past |
| `ListFormat` | `join` — "and", "or" and unit lists |
| `Segmenter` | `words`, `sentences` |
| `Locale` | `negotiate`, `resolve` |

Their signatures, options shapes and enums are ADR 0277 § 1's table. No member reaches another's job,
and there is no per-value member beside a batch one (`rule:core-api/one-paradigm-per-operation`).
Message formatting is not in the roster until ICU4X's MessageFormat 2 is stable, and grapheme
segmentation, case mapping and normalization stay in `Core\Str`.

`Core\Cldr::pluralCategory` and `ordinalCategory` stay in Tier 0 beside `PluralRules`. On a bare
language tag from `Core\Cldr`'s roster the two return the same category for every number; `PluralRules`
also reads a region, so `pt-PT` may differ from `Core\Cldr`'s `pt`.

**Not on disk.** The component crate `extensions/intl/` and its Novis half do not exist; `wit/intl.wit`
is the interface they will export.
