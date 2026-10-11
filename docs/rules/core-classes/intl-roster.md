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

Their signatures are the classes in `extensions/intl/nvs/`, and their options shapes and enums are
`wit/intl.wit`'s. No member reaches another's job, and there is no per-value member beside a batch one
(`rule:core-api/one-paradigm-per-operation`). Message formatting is not in the roster until ICU4X's
MessageFormat 2 is stable, and grapheme segmentation, case mapping and normalization stay in `Core\Str`.

A manifest names exactly one class, so `Icu` is the boundary and the job classes a program uses are
Novis source in the same `.nvsx`, as the image component's are. The Novis half does the two things the
guest cannot: it converts a `Core\Time` value to local fields (`rule:core-classes/intl-host-converts-time`),
and it writes a number as its decimal text.

- **Two collation members, two results.** `sort` returns the strings in order; `sortKeys` returns bytes
  a program stores in a column or sorts rows by. A key is valid only under the ICU4X version the
  component pins, so a stored key is rebuilt when the binary's component version changes.
- **A number crosses as its decimal text.** The text is exact, so a `decimal` amount is never rounded
  through `f64`, and the export has one numeric type. A `float` that is not finite throws
  `RuntimeError` in the Novis half before the call, and a currency code that is not three ASCII letters
  throws `LogicError`. The four `NumberFormat` members are four styles with four options shapes.
- **One plural category type.** A catalog keyed on `Core\Cldr`'s category reads `PluralRules`'s with no
  conversion.

`Core\Cldr::pluralCategory` and `ordinalCategory` stay in Tier 0 beside `PluralRules`. On a bare
language tag from `Core\Cldr`'s roster the two return the same category for every number; `PluralRules`
also reads a region, so `pt-PT` may differ from `Core\Cldr`'s `pt`. Neither is deprecated: `Core\Cldr`
is the one a template reaches without the sandbox, and `PluralRules` is the one that knows every
locale.

- **Memory:** nothing until a process first calls the component; then the compiled module, shared by
  every core, and each calling request's instance under that request's cap. The CLDR data is in the
  binary, and the component's module doc states its size. If that size is too large, the answer is a
  slimmer locale set, never an optional component.

**Not on disk.** The component crate `extensions/intl/` and its Novis half do not exist; `wit/intl.wit`
is the interface they will export.

**Declined:**

- **Numbers as a `variant` of `s64`, `f64` and digits.** It adds three cases to every numeric export to
  carry what one string already carries exactly.
- **`RelativeTime` from a `Duration`.** A month and a year are not a fixed number of nanoseconds, so the
  unit would be a guess. The program chooses the unit and the count.
