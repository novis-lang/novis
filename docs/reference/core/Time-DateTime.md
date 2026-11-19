---
summary: a civil date and time in a zone — calendar arithmetic and CLDR formatting
keywords: date, time, datetime, format a date, add days, add months, date difference, weekday, day of year, leap year, immutable date, CLDR, format pattern, calendar arithmetic, DST, next monday, last day of month
---

A `DateTime` is a civil reading — year, month, day, hour, minute, second, nanosecond — in one `Zone`,
built by `Core\Time::at` or `Core\Time::parse` or read off an `Instant` with `->in($zone)`. It is
immutable: every member answers a new value. Its arithmetic is calendar arithmetic: `plus(1,
Core\Unit::Month)` keeps the day-of-month clamped to the new month's length, `plus(1, Core\Unit::Day)`
across a DST boundary is a 23- or 25-hour day, and `difference` counts whole units in the receiver's
zone. `format` takes a **CLDR pattern**. `Core\Time::parse` reads the same patterns, and `Date::format`
and `TimeOfDay::format` use them for their own fields. A pattern written directly
in the code is checked when the program compiles, so a letter outside the table below is a compile error; a pattern
computed at run time throws a `LogicError` instead. Month and weekday names are always in English.

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
var $utc = Core\Time\Zone::UTC;
var $d = Core\Time::at(2024, 1, 31, $utc, { hour: 9, minute: 30 });
echo $d->format('yyyy-MM-dd HH:mm:ss'), "\n";
echo $d->format("EEEE, d MMMM yyyy 'at' h:mm a"), "\n";

echo $d->plus(1, Core\Unit::Month)->format('yyyy-MM-dd'), "\n";
echo $d->minus(1, Core\Unit::Week)->format('yyyy-MM-dd'), "\n";
echo $d->next(Core\Time\DateTime::Weekday::Monday)->format('yyyy-MM-dd HH:mm'), "\n";
echo $d->with({ month: 3, day: 1 })->format('yyyy-MM-dd'), "\n";
echo $d->startOf(Core\Unit::Day)->format('HH:mm'), ' ',
    $d->endOf(Core\Unit::Month)->format('dd HH:mm:ss.SSS'), "\n";

echo $d->difference(Core\Time::at(2026, 8, 30, $utc), Core\Unit::Year), "\n";
echo $d->weekday() == Core\Time\DateTime::Weekday::Wednesday ? 'midweek' : 'other', "\n";
echo $d->dayOfYear(), ' ', $d->isLeapYear() ? 'leap' : 'common', "\n";
echo $d->toInstant()->toIso(), "\n";
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
```
