---
summary: a civil date and time in a zone — calendar arithmetic and CLDR formatting
keywords: date, time, datetime, create a date, parse a date, ISO 8601, format a date, add days, add months, date difference, weekday, day of year, leap year, immutable date, CLDR, format pattern, calendar arithmetic, DST, next monday, last day of month
---

A `DateTime` is a date and a time of day in one `Zone`. `DateTime::fromIso` reads a text such as
`2024-03-01 09:30` in the zone you give it. A date alone, such as `2024-03-01`, is the start of
that day. `DateTime::at($date, $time, $zone)` builds one from a `Date` and a `TimeOfDay`.
`DateTime::parse` reads text in a pattern of your choice. `->in($zone)` gives the `DateTime` of an
`Instant`.

A zone that is fixed in the code is written as text, such as `'Europe/Berlin'`. It is checked when
the program compiles. A zone name that arrives while the program runs needs `Core\Time\Zone::of`.

A `DateTime` never changes: every method returns a new value. Its arithmetic follows the calendar.
`plus(1, Core\Unit::Month)` keeps the day inside the new month. `plus(1, Core\Unit::Day)` across a
daylight saving change is 23 or 25 hours. `difference` counts whole units in the zone of the value
it is called on.

`year()`, `month()`, `day()`, `hour()`, `minute()`, `second()` and `nanoseconds()` return the fields as
numbers. Each one reads the clock in the value's own zone. `year()` returns an `int`, and the others
return a `uint`. These are the same names and types that `with` takes. `daysInMonth()` returns the
number of days in the month, and `quarter()` returns `1` to `4`. `isoWeek()` returns the ISO 8601
week number, and `isoWeekYear()` returns the year that this week belongs to. These two can differ
from `year()` for a few days around New Year.

`toIso()` writes the value as text with its offset and its zone, such as
`2026-01-05T08:30:00+01:00[Europe/Vienna]`. `DateTime::fromIso` reads this text back with no second
argument. `compareTo` checks which of two values is earlier, and `<`, `>`, `<=`, `>=` use it too. It
compares the moment, so 10:00 in `Europe/Berlin` and 09:00 in `Europe/London` on one winter day are
equal. `==` compares the moment in the same way.

`format` takes a **CLDR pattern**. `DateTime::parse` reads the same patterns, and `Date::format`
and `TimeOfDay::format` use them for their own fields. A pattern written in the code is
checked when the program compiles, so a letter outside the table below is a compile error. A pattern
computed at run time throws a `LogicError`. Month and weekday names are always in English.

| Letter | Field | Counts |
|---|---|---|
| `G` | era | `G`–`GGG` `AD` or `BC`, `GGGG` `Anno Domini`, `GGGGG` `A` |
| `y` / `U` | year, such as `-0043` before year 1 | `yy` two digits; any other count zero-pads to it |
| `u` / `r` | year | zero-pads to the count; `parse` also reads a `-` with these |
| `Y` | the year the `w` week belongs to | as `y` |
| `Q` / `q` | quarter | `Q`/`QQ` numeric, `QQQ` `Q1`, `QQQQ` `1st quarter`, `QQQQQ` `1` |
| `M` / `L` | month | `M`/`MM` numeric, `MMM` `Mar`, `MMMM` `March`, `MMMMM` `M` |
| `w` | week of the year | ISO 8601 weeks: a week starts on Monday |
| `W` | week of the month | the same rule; `0` for a short first week |
| `d` | day of month | numeric, zero-padded to the count |
| `D` | day of year | numeric |
| `F` | which of that weekday in the month | `1` for the first Friday of the month |
| `g` | modified Julian day | numeric |
| `E` | weekday | `E`–`EEE` `Tue`, `EEEE` `Tuesday`, `EEEEE` `T`, `EEEEEE` `Tu` |
| `e` / `c` | weekday | `e`/`ee`/`c` the number, Monday `1`; three or more letters as `E` |
| `a` | AM/PM | any count |
| `b` | AM/PM, or `noon` and `midnight` at those times | 1–4 the name, 5 short |
| `B` | part of the day, such as `in the morning` | 1–4 the name, 5 short |
| `h` / `H` / `K` / `k` | hour 1–12 / 0–23 / 0–11 / 1–24 | numeric |
| `m` | minute | numeric |
| `s` | second | numeric |
| `S` | fractional second | one digit per count, truncated |
| `A` | milliseconds since the start of the day | numeric, zero-padded to the count |
| `X` | offset, `Z` at zero | `X` `+01`, `XX` `+0100`, `XXX` `+01:00` |
| `x` | offset, never `Z` | the same three counts |
| `Z` | offset | `Z`–`ZZZ` `+0100`, `ZZZZ` `GMT+01:00`, `ZZZZZ` as `XXX` |
| `O` | offset after `GMT` | `O` `GMT+1`, `OOOO` `GMT+01:00` |
| `z` | offset after `GMT` | `z`–`zzz` `GMT+1`, `zzzz` `GMT+01:00` |
| `v` | offset after `GMT` | `v` `GMT+1`, `vvvv` `GMT+01:00` |
| `VV` | the zone's IANA identifier | `VV` only |

Text between `'…'` is printed as it is, and `''` is one apostrophe. These letters do not compile: `C`,
`f`, `i`, `I`, `j`, `J`, `l`, `n`, `N`, `o`, `p`, `P`, `R`, `t` and `T`.

```nvs
<?nvs
var $d = Core\Time\DateTime::fromIso('2024-01-31 09:30', 'UTC');
echo $d->format('yyyy-MM-dd HH:mm:ss'), "\n";
echo $d->format("EEEE, d MMMM yyyy 'at' h:mm a"), "\n";

echo $d->plus(1, Core\Unit::Month)->format('yyyy-MM-dd'), "\n";
echo $d->minus(1, Core\Unit::Week)->format('yyyy-MM-dd'), "\n";
echo $d->next(Core\Time\Weekday::Monday)->format('yyyy-MM-dd HH:mm'), "\n";
echo $d->with({ month: 3, day: 1 })->format('yyyy-MM-dd'), "\n";
echo $d->startOf(Core\Unit::Day)->format('HH:mm'), ' ',
    $d->endOf(Core\Unit::Month)->format('dd HH:mm:ss.SSS'), "\n";

echo $d->difference(Core\Time\DateTime::fromIso('2026-08-30', 'UTC'), Core\Unit::Year), "\n";
echo $d->weekday() == Core\Time\Weekday::Wednesday ? 'midweek' : 'other', "\n";
echo $d->dayOfYear(), ' ', $d->isLeapYear() ? 'leap' : 'common', "\n";
echo $d->toInstant()->toIso(), "\n";

var $eve = Core\Time\DateTime::at(Core\Time\Date::at(2024, 3, 30), Core\Time\TimeOfDay::at(12, 0), 'Europe/Berlin');
echo $eve->plus(1, Core\Unit::Day)->format('yyyy-MM-dd HH:mm XXX'), "\n";
echo Core\Time\DateTime::parse('5 March 2024', 'd MMMM yyyy', 'UTC')->format('yyyy-MM-dd HH:mm'), "\n";
```
```output
2024-01-31 09:30:00
Wednesday, 31 January 2024 at 9:30 AM
2024-02-29
2024-01-24
2024-02-05 09:30
2024-03-01
00:00 31 23:59:59.999
2
midweek
31 leap
2024-01-31T09:30:00Z
2024-03-31 12:00 +02:00
2024-03-05 00:00
```
