---
summary: a civil date and time in a zone — calendar arithmetic and CLDR formatting
keywords: date, gmdate, idate, strftime, date_format, date_add, date_sub, modify, strtotime, setDate, setTime, setISODate, date_diff, DateInterval, date("N"), date("z"), date("L"), DateTimeImmutable, CLDR, format pattern, calendar arithmetic, DST, next monday, last day of month
---

A `DateTime` is a civil reading — year, month, day, hour, minute, second, nanosecond — in one `Zone`,
built by `Core\Time::at` or `Core\Time::parse` or read off an `Instant` with `->in($zone)`. It is
immutable: every member answers a new value. Its arithmetic is calendar arithmetic: `plus(1,
Core\Unit::Month)` keeps the day-of-month clamped to the new month's length, `plus(1, Core\Unit::Day)`
across a DST boundary is a 23- or 25-hour day, and `difference` counts whole units in the receiver's
zone. `format` takes a **CLDR pattern**, not PHP's `date()` letters — the same grammar `Core\Time::parse`
reads and `Date::format` and `TimeOfDay::format` take for their own fields. A pattern written directly
in the code is checked when the program compiles, so a letter outside the table below is a compile error; a pattern
computed at run time throws a `LogicError` instead. Names render in English only; there is no
`setlocale`.

| Letter | Field | Counts |
|---|---|---|
| `y` | year | `yy` two digits; any other count zero-pads to it |
| `M` | month | `M`/`MM` numeric, `MMM` `Mar`, `MMMM` `March`, `MMMMM` `M` |
| `d` | day of month | numeric, zero-padded to the count |
| `D` | day of year | numeric |
| `E` | weekday | `E`–`EEE` `Tue`, `EEEE` `Tuesday`, `EEEEE` `T`, `EEEEEE` `Tu` |
| `a` | AM/PM | any count |
| `h` / `H` / `K` / `k` | hour 1–12 / 0–23 / 0–11 / 1–24 | numeric |
| `m` | minute | numeric |
| `s` | second | numeric |
| `S` | fractional second | one digit per count, truncated |
| `X` | offset, `Z` at zero | `X` `+01`, `XX` `+0100`, `XXX` `+01:00` |
| `x` | offset, never `Z` | the same three counts |
| `VV` | the zone's IANA identifier | `VV` only |

Text between `'…'` is printed as it is, and `''` is one apostrophe. `G`, `Q`, `w`, `W`, `L`, `c`, `F` and `u`
are refused.

```nvs
<?nvs
var $utc = Core\Time\Zone::UTC;
var $d = Core\Time::at(2024, 1, 31, $utc, {hour: 9, minute: 30});
echo $d->format("yyyy-MM-dd HH:mm:ss"), "\n";
echo $d->format("EEEE, d MMMM yyyy 'at' h:mm a"), "\n";

echo $d->plus(1, Core\Unit::Month)->format("yyyy-MM-dd"), "\n";
echo $d->minus(1, Core\Unit::Week)->format("yyyy-MM-dd"), "\n";
echo $d->next(Core\Weekday::Monday)->format("yyyy-MM-dd HH:mm"), "\n";
echo $d->with({month: 3, day: 1})->format("yyyy-MM-dd"), "\n";
echo $d->startOf(Core\Unit::Day)->format("HH:mm"), " ",
    $d->endOf(Core\Unit::Month)->format("dd HH:mm:ss.SSS"), "\n";

echo $d->difference(Core\Time::at(2026, 8, 30, $utc), Core\Unit::Year), "\n";
echo $d->weekday() == Core\Weekday::Wednesday ? "midweek" : "other", "\n";
echo $d->dayOfYear(), " ", $d->isLeapYear() ? "leap" : "common", "\n";
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
