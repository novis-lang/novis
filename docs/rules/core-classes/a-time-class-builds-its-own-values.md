Each `Core\Time` value class builds its own values, and `Core\Time` itself is the clock. A constructor
is a static member of the class it returns, so the class a reader looks up is the class that makes it:

| Class | Static members |
|---|---|
| `Core\Time` | `now()`, `monotonic()`, `sleep(Duration)` |
| `Core\Time\Instant` | `fromEpoch(int, {nanos?})`, `fromIso(text)` |
| `Core\Time\Date` | `at(int, uint, uint)`, `fromIso(text)` |
| `Core\Time\TimeOfDay` | `at(uint, uint, {second?, nanos?})`, `fromIso(text)` |
| `Core\Time\DateTime` | `at(Date, TimeOfDay, Zone)`, `fromIso(text, Zone)`, `parse(text, pattern, Zone)` |

`DateTime::at` takes its three parts in the order the readers `date()`, `timeOfDay()` and `zone()`
return them, and resolves a skipped local time forward and a repeated one to the earlier instant.
`now` stays on `Core\Time` because it reads the outside world, as `monotonic` and `sleep` do, and the
determinism rules fake the clock through it (`rule:testing/determinism-declared-on-the-test`).
`TimeOfDay::at`'s option bag holds only finer parts of the same value. There is no
`Date::atStartOfDay`: `DateTime::at($date, TimeOfDay::at(0, 0), $zone)` is the same value, resolved
the same way, and `DateTime::fromIso('2024-03-01', $zone)` is its short form.

**One text format per class, the ISO 8601 extended form, and nothing else.**

- `Date::fromIso` reads `YYYY-MM-DD` over the year range `Date::at` takes. No week date, no ordinal
  date and no basic form such as `20240301`.
- `TimeOfDay::fromIso` reads `HH:MM`, `HH:MM:SS` or `HH:MM:SS.f` with one to nine fraction digits. No
  `24:00` and no leap second `60`.
- `DateTime::fromIso(text, Zone)` reads a date, or a date, a separator and a time. The separator is
  `T` or one space, as RFC 3339 allows. A date alone is the first instant of that day, and a local
  time resolves as `DateTime::at` resolves it. An offset or a `Z` in the text throws, and the message
  names `Instant::fromIso(...)->in($zone)`: the zone is the argument, never the text.
- `Instant::fromIso` reads an ISO 8601 timestamp that carries its own offset or `Z`, and takes the
  same two separators.
- A text outside the format throws `ParseError` naming the part that is wrong.

**A literal text is checked while compiling.** A string literal at the text argument of every `fromIso`
is a `rule:expressions/intrinsic-constant-arguments` intrinsic, one `Grammar::Iso` row per class, so
`Date::fromIso('2024-02-30')` is a compile error at the literal. The run-time path runs the same parser
(`rule:expressions/preparation-preserves-behaviour`). `DateTime::parse` keeps its prepared CLDR
pattern row under its new owner.

**A zone fixed in the source is written as text.** A string literal at a parameter typed
`Core\Time\Zone` is that zone: `DateTime::fromIso('2024-03-01 09:30', 'Europe/Berlin')`,
`$instant->in('UTC')`. It is the pattern of a written duration (`rule:types/duration`): the literal
for a value fixed in the source, and the member, `Zone::of($text)`, for text that arrives at run time,
which is the one place a zone name throws. The checker reads the literal as it reads a key literal at
a `property<C>` parameter (`rule:types/property-key`): a name the time-zone database lacks, or an
offset `Zone::fixed` does not take, is a compile error at the literal. A name is looked up as
`Zone::of` looks it up, case ignored, and kept as written, so the literal refuses only what
`Zone::of` throws on. Only a literal converts. A
`string` variable at a `Zone` parameter stays a type error, so tainted text reaches a zone only through
`Zone::of`. `Zone::of('UTC')` written with a literal still compiles, and the docs show the short form.
The editor completes a string at a `Zone` parameter from the same database, one `/` segment at a time.

**A written zone is built once, never per use.** Each written zone is one immortal instance in the
compiled unit's data section, the mechanism a written duration and a fixed `` html`…` `` use. Its one
slot, the zone id, is an immortal string, and no `Zone` member writes a slot, which is what makes the
instance safe to share across requests and cores. `Zone::UTC`, and `Zone::of` or `Zone::fixed` called
with a literal, fold to the same kind of instance. Each written zone is its own instance, so `==` stays
identity (`rule:expressions/object-identity-equality`).

**What it spends.** A written zone costs a header, one slot and its id's bytes once, charged to the
compiled unit and freed with it, and nothing per use; `Zone::of` costs an object per call. A literal
`fromIso` is checked once while compiling, and reading it at run time is a fixed-format scan, cheaper
than a CLDR pattern. `DateTime::fromIso('…', 'UTC')` spends one `DateTime` and nothing else.
`DateTime::at` from computed parts spends a `Date` and a `TimeOfDay` before the `DateTime`: two small
objects per call more than the single eight-argument constructor it replaces, spent to buy one
constructor per class with no positional seven-field signature.

A member that moved here from `Core\Time` is a compile error naming its new place
(`rule:core-api/a-moved-name-says-where-it-went`), with no alias keeping the old name
(`rule:core-api/no-deprecation-window-before-a-release`).

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
- **One shared instance per zone name across written zones.** It makes `==` true for two written zones
  and false for one `Zone::of` returns, for no saving a request sees.
