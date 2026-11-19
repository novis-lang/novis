---
summary: a calendar date with no time and no zone
keywords: date, calendar date, day-month-year, zone-free, valid date, days in month, leap year
---

A `Date` is a year, a month and a day. It has no time of day and no zone. `Date::at` builds one, and
`->date()` gives the date of a `DateTime`. To read a date from text, use `Core\Time::parse` with a
pattern and a zone, and call `->date()` on the result. A date that does not exist throws an error
when it is built.

`year()`, `month()` and `day()` return the three fields as numbers. `weekday()` returns a
`Core\Time\DateTime::Weekday` case, `dayOfYear()` returns the day of the year from `1`, and
`isLeapYear()` checks the year. `atStartOfDay($zone)` returns a `DateTime` at the first moment of
the date in that zone. This is usually midnight. Add a time of day to it with `withTime`.

`plus` and `minus` move a date by `Core\Unit::Day` or a larger unit. A smaller unit throws an
error. Moving by months keeps the day inside the new month, as `DateTime::plus` does. `format`
takes the date letters of the patterns `DateTime::format` uses. A time letter or a zone letter
does not compile, because a `Date` has no time and no zone. `<`, `>`, `<=`, `>=` and `compareTo`
order two dates.

```nvs
<?nvs
var $d = Core\Time\Date::at(2024, 1, 31);
echo $d->format('yyyy-MM-dd'), "\n";
echo $d->format('EEEE, d MMMM'), "\n";

echo $d->plus(1, Core\Unit::Month)->format('yyyy-MM-dd'), "\n";
echo $d->minus(1, Core\Unit::Day)->format('EEE d MMM'), "\n";
echo $d->with({ month: 3 })->format('yyyy-MM-dd'), "\n";
echo $d->compareTo(Core\Time\Date::at(2024, 2, 1)), "\n";

try {
    Core\Time\Date::at(2023, 2, 29);
} catch (RuntimeError $bad) {
    echo "2023-02-29 does not exist\n";
}

var $utc = Core\Time\Zone::UTC;
echo Core\Time::at(2024, 3, 1, $utc, { hour: 23 })->date()->format('d/M/yyyy'), "\n";

echo $d->year(), " ", $d->month(), " ", $d->day(), "\n";
echo $d->dayOfYear(), " ", $d->isLeapYear() ? "leap year" : "not a leap year", "\n";
echo $d->atStartOfDay(Core\Time\Zone::of('Europe/Berlin'))->format('yyyy-MM-dd HH:mm XXX'), "\n";
```
```output
2024-01-31
Wednesday, 31 January
2024-02-29
Tue 30 Jan
2024-03-31
-1
2023-02-29 does not exist
1/3/2024
2024 1 31
31 leap year
2024-01-31 00:00 +01:00
```
