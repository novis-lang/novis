Each `Core\Time` value class builds its own values, and `Core\Time` itself is the clock. A constructor
is a static member of the class it returns, so the class a reader looks up is the class that makes it:

| Class | Static members |
|---|---|
| `Core\Time` | `now()`, `monotonic()`, `sleep(Duration)` |
| `Core\Time\Instant` | `fromEpoch(int, {nanoseconds?})`, `fromIso(text)` |
| `Core\Time\Date` | `at(int, uint, uint)`, `fromIso(text)`, `parse(text, pattern)`, `range(Date, Date, {step?})` |
| `Core\Time\TimeOfDay` | `at(uint, uint, {second?, nanoseconds?})`, `fromIso(text)` |
| `Core\Time\DateTime` | `at(Date, TimeOfDay, Zone)`, `fromIso(text, ?Zone)`, `parse(text, pattern, Zone)` |

`DateTime::at` takes its three parts in the order the readers `date()`, `timeOfDay()` and `zone()`
return them, and resolves a skipped local time forward and a repeated one to the earlier instant.
`now` stays on `Core\Time` because it reads the outside world, as `monotonic` and `sleep` do, and the
determinism rules fake the clock through it (`rule:testing/determinism-declared-on-the-test`).
`TimeOfDay::at`'s option bag holds only finer parts of the same value. There is no
`Date::atStartOfDay`: `DateTime::at($date, TimeOfDay::at(0, 0), $zone)` is the same value, resolved
the same way, and `DateTime::fromIso('2024-03-01', $zone)` is its short form.

`Date::parse` reads a date through a CLDR pattern of date letters only, because a `Date` has no time
of day and no zone to keep. `Date::range` lists the dates from its first argument to its second, both
included, and counts down when the first is later, as `Core\Arr::range` does with its option name
`step`. The step is `Day`, `Week`, `Month`, `Quarter` or `Year`, a closed subset in the signature
(`rule:types/single-value-types`). Entry `k` is the first date plus `k` steps, so a month step from
the 31st gives each shorter month's last day, as `Date::plus` does. **What it spends:** one `Date`
per entry and the list, charged to the request, and the list is asked of the memory ceiling once.

**An epoch reader floors.** `toEpochSeconds`, `toEpochMillis` and `toEpochMicros` give the largest
whole count at or before the instant, so `fromEpoch` of the count and the remainder builds the same
instant. Half a second before 1970 is second `-1`, millisecond `-500`. Before 1970, an instant with a
part of a unit reads one less than a reader that truncates toward zero gives.

`Date::at` names its parameters `year`, `month` and `day`, the names of its readers.

**One text format per class, the ISO 8601 extended form, and nothing else.** Each class writes the
text its `fromIso` reads: `Date::toIso`, `TimeOfDay::toIso`, `DateTime::toIso` and `Instant::toIso`,
and `fromIso($v->toIso())` is `$v` at both ends of the range.

- `Date::fromIso` reads `YYYY-MM-DD` over the year range `Date::at` takes. No week date, no ordinal
  date and no basic form such as `20240301`. Year zero is `0000`, and `-0000` throws.
- `TimeOfDay::fromIso` reads `HH:MM`, `HH:MM:SS` or `HH:MM:SS.f` with one to nine fraction digits. No
  `24:00` and no leap second `60`.
- `DateTime::fromIso(text, ?Zone)` reads a date, or a date, a separator and a time. The separator is
  `T` or one space, as RFC 3339 allows. After the time the text may write an offset, `±HH:MM` or
  `±HH:MM:SS`, and then one zone in brackets, as RFC 9557 writes a zoned time:
  `2026-01-05T08:30:00+01:00[Europe/Vienna]`. A text with `[Zone]` needs no `$zone`; a text without
  one is read in `$zone` and throws when it is `null`. A text with `[Zone]` and a `$zone` naming
  another zone throws (`rule:errors/ambiguous-input-refused`). A date alone is the first instant of
  that day, and a local time with no offset resolves as `DateTime::at` resolves it. An offset picks
  one copy of a repeated hour, and an offset the zone does not have at that local time throws. An
  offset with no zone after it, or a `Z`, throws and the message names
  `Instant::fromIso(...)->in($zone)`. Only one bracket is read: a critical flag `[!...]` and a tag
  such as `[u-ca=...]` throw. A zone name is looked up as `Zone::of` looks it up, and an offset in
  the brackets is a fixed zone, `[+05:30]`.
- `DateTime::toIso` writes that full form: the local time with seconds, a fraction only when it is
  not zero, the offset as `±HH:MM` (`±HH:MM:SS` for a local mean time with seconds) and the zone's
  id. UTC is `+00:00[UTC]`, never `Z`. **Why the offset and the zone are both in the text:** the
  offset names one moment in a repeated hour, so the text is never ambiguous, and the zone lets the
  text cross a query string or a JSON document with no second argument and come back in its own
  zone. The text without a zone is `format(...)`'s.
- `Instant::fromIso` reads a date, `T` or one space, a time as `TimeOfDay::fromIso` reads it, and
  `Z` or a `±HH:MM` offset. A leap second `60`, a lower-case `t` or `z`, a comma before the fraction
  and a basic offset such as `+0100` throw.
- A year is four digits, with a `-` before a year before year 1. The expanded `±YYYYYY` form throws,
  so there is one text per year.
- `Date::toIso` writes `YYYY-MM-DD`, and `TimeOfDay::toIso` writes `HH:MM:SS`, with a fraction only
  when it is not zero.
- `Instant::toIso` writes RFC 3339 in UTC with the year in that same form, so the first instant is
  `-9999-01-02T01:59:59Z` and `fromIso($i->toIso())` is `$i` at both ends of the range. JSON writes
  and reads an `Instant` as that text.
- A text outside the format throws `ParseError` naming the part that is wrong.

**A literal text is checked while compiling.** A string literal at the text argument of every `fromIso`
is a `rule:expressions/intrinsic-constant-arguments` intrinsic, one `Grammar::Iso` row per class, so
`Date::fromIso('2024-02-30')` is a compile error at the literal. A `DateTime` literal's zone name
and its offset are checked too, and a literal with no `[Zone]` in a call that writes no `$zone` does
not compile. The run-time path runs the same parser
(`rule:expressions/preparation-preserves-behaviour`). `DateTime::parse` keeps its prepared CLDR
pattern row under its new owner, and `Date::parse` has one too, which refuses a time or zone letter.

**A written pattern with `Y` and no `w` warns.** `Y` is the year of the ISO week, which differs from
the calendar year `y` on a few days around New Year: on 2024-12-30, `YYYY` writes `2025`. So a literal
pattern at `DateTime::format`, `DateTime::parse` or `Date::parse` with a `Y` and no `w` is warning `W1038`, whose
help names `y`. A warning and not an error, because `YYYY` beside `ww` is correct and a program may
mean `Y` alone. A pattern built at run time is not judged.

**A zone fixed in the source is written as text.** A string literal at a parameter typed
`Core\Time\Zone` is that zone: `DateTime::fromIso('2024-03-01 09:30', 'Europe/Berlin')`,
`$instant->in('UTC')`. It is the pattern of a written duration (`rule:types/duration`): the literal
for a value fixed in the source, and the member, `Zone::of($text)`, for text that arrives at run time,
which is the one place a zone name throws. The checker reads the literal as it reads a key literal at
a `property<C>` parameter (`rule:types/property-key`): a name the time-zone database lacks, or an
offset `Zone::fixed` does not take, is a compile error at the literal. A name is looked up as
`Zone::of` looks it up, case ignored, and kept as the database writes it, so `'europe/berlin'` and
`Zone::of('EUROPE/BERLIN')` are both `Europe/Berlin`, and the literal refuses only what `Zone::of`
throws on. An offset is one sign, one or two hour digits, and two digits for each later part, so
`'+-5:00'` does not compile. Only a literal converts. A
`string` variable at a `Zone` parameter stays a type error, so tainted text reaches a zone only through
`Zone::of`. `Zone::of('UTC')` written with a literal still compiles, and the docs show the short form.
The editor completes a string at a `Zone` parameter from the same database, one `/` segment at a time.

**A written zone is built once, never per use.** Each written zone is one immortal instance in the
compiled unit's data section, the mechanism a written duration and a fixed `` html`…` `` use. Its one
slot, the zone id, is an immortal string, and no `Zone` member writes a slot, which is what makes the
instance safe to share across requests and cores. `Zone::UTC`, and `Zone::of` or `Zone::fixed` called
with a literal, fold to the same kind of instance. Each written zone is its own instance, and `==` compares
a zone by its id (`rule:expressions/object-identity-equality`), so two written zones with one id are
`==`, and so is the zone `Zone::of` returns for that id.

**What it spends.** A written zone costs a header, one slot and its id's bytes once, charged to the
compiled unit and freed with it, and nothing per use; `Zone::of` costs an object per call. A literal
`fromIso` is checked once while compiling, and reading it at run time is a fixed-format scan, cheaper
than a CLDR pattern. `DateTime::fromIso('…', 'UTC')` spends one `DateTime` and nothing else.
`DateTime::at` from computed parts spends a `Date` and a `TimeOfDay` before the `DateTime`: two small
objects per call more than the single eight-argument constructor it replaces, spent to buy one
constructor per class with no positional seven-field signature. Measured (`data/perf/members.md`),
`DateTime::at` over a built `Date` and `TimeOfDay` is 5 allocations and 267 bytes per call, beside
the old `Core\Time::at` row's 4 and 210. `Date::at` adds one allocation and 80 bytes, and
`TimeOfDay::at` one and 96. A call built from computed parts is then about 7 allocations and 443
bytes. `DateTime::fromIso` is 4 allocations and 218 bytes.

A member that moved here from `Core\Time` has no alias keeping the old name, and the old name is an
unknown name (`rule:core-api/no-deprecation-window-before-a-release`).

**Declined:**

- **A free-form date text in PHP's manner**, such as `strtotime` or a parser that guesses the format.
  One text reads one way, and a guess that succeeds on a wrong reading is a silent bug.
- **`Core\Time` as the home of every constructor.** A reader looks for a `Date` maker on `Date`, and a
  catch-all class mixes the clock, which reads the world, with pure constructors.
- **An instance `Date->at(TimeOfDay, Zone)`.** `Date::at` is already the static constructor, and one
  name with two meanings is a distinction every call site must make.
- **A positional seven-field `DateTime::at`.** Seven integers in a row are easy to swap, and the parts
  already have classes that check themselves.
- **Keeping `Date::atStartOfDay`.** It is a second way to one value
  (`rule:core-api/one-paradigm-per-operation`).
- **A `string` zone parameter in place of `Zone`.** Every member would check the name again and gain an
  error path, and tainted text would reach a zone without passing `Zone::of`.
- **One shared instance per zone name across written zones.** `==` already compares a zone by its id,
  so sharing changes no result, and it saves one small object per written zone, charged to the
  compiled unit, for no saving a request sees.
- **A cache from a zone's id to its `TimeZone`.** Every conversion looks the id up again, and it was
  measured before anything changed: one lookup of a name already loaded is about 21 ns and allocates
  nothing, so the two an `->in($zone)` makes are about 2% of its recorded 1,823 ns. A cache would be
  state on the request path for no saving a request sees.
