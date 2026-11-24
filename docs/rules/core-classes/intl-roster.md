The intl component's classes are `Collator`, `NumberFormat`, `PluralRules`, `DateFormat`,
`RelativeTime`, `ListFormat`, `Segmenter` and `Locale`, all under `Novis\Intl` and written in Novis
source over one manifest class, `Novis\Intl\Icu`, whose static methods are the exports of
`nvs:intl/icu` in `wit/intl.wit`.

| Class | Members |
|---|---|
| `Collator` | `sort`, `sortKeys` — the strings in the locale's order, or one byte key per string that orders the same way |
| `NumberFormat` | `decimal`, `percent`, `currency`, `compact` |
| `PluralRules` | `cardinal`, `ordinal` — each returns `Core\Cldr::PluralCategory` |
| `DateFormat` | `dateTimes`, `dates`, `times` — over `Core\Time\DateTime`, `Date` and `TimeOfDay` |
| `RelativeTime` | `format` — a count and a `RelativeTime::Unit`, negative in the past |
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

**`DateFormat` picks its parts with `Fields`, and the locale picks their order.** `dates` and
`dateTimes` take `fields?: Fields` beside `length`. Its ten cases name the parts from the largest to
the smallest: `Day`, `MonthDay`, `YearMonthDay` (the default), `Weekday`, `DayWeekday`,
`MonthDayWeekday`, `YearMonthDayWeekday`, `Month`, `YearMonth` and `Year`. A name says which parts
appear, never their order, so `YearMonth` in `de` is `Januar 2026`. `Month`, `YearMonth` and `Year` on
`dateTimes` throw `LogicError` naming `dates` before anything is formatted, because ICU4X has no field
set that writes a time beside a date with no day, and dropping the time would hide what the caller
asked for. Both members keep one options type. `format-dates` takes a `date-options` record,
`{ length, fields }`, as `format-date-times` takes its options. **What it spends:** `Month`,
`YearMonth` and `Year` load the number formatter once per call, because ICU4X does not load it for
some calendar-period patterns, such as `pt`'s long year and month. The other seven cases spend
nothing more.

Each case of an intl enum is described in the enum's one `help` text, so every enum of the component
names each of its cases. The manifest has no help field per case: that would change every
component's manifest and `nvs meta` for one card per case.

- **Memory:** nothing until a process first calls the component; then the compiled module, shared by
  every core, and each calling request's instance under that request's cap. The CLDR data is in the
  binary, and the component's module doc states its size. If that size is too large, the answer is a
  slimmer locale set, never an optional component.

The component crate is `extensions/intl/`, its Novis half is `extensions/intl/nvs/`, and `wit/intl.wit`
is the interface it exports.

**Declined:**

- **Numbers as a `variant` of `s64`, `f64` and digits.** It adds three cases to every numeric export to
  carry what one string already carries exactly.
- **`RelativeTime` from a `Duration`.** A month and a year are not a fixed number of nanoseconds, so the
  unit would be a guess. The program chooses the unit and the count.
- **A `Full` length on `DateFormat`.** ICU4X 2.4 has `Long`, `Medium` and `Short` only.
  `YearMonthDayWeekday` with `Long` is the long form with the weekday.
- **CLDR skeleton strings in place of `Fields`.** A fifth grammar beside the closed four
  (`rule:core-api/no-mode-strings`).
