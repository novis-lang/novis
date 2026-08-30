---
summary: a calendar date with no time and no zone
keywords: checkdate, date, cal_days_in_month, calendar date, day-month-year, zone-free
---

A `Date` is the year, month and day alone — no time of day and no zone — built by `Date::at` or read
off a `DateTime` with `->date()`. A date that does not exist throws where it is built, which is all
`checkdate` did. It steps by `Core\Unit::Day` and larger, with the same month-end clamping as
`DateTime::plus`; a smaller unit is refused. `format` takes the date letters of the CLDR grammar
`DateTime::format` uses, and a time or zone letter in the pattern is refused because a `Date` has
none to render. Two dates are ordered with `compareTo`; the `<` operators do not compile on a `Date`.

```nvs
<?nvs
var $d = Core\Time\Date::at(2024, 1, 31);
echo $d->format("yyyy-MM-dd"), "\n";
echo $d->format("EEEE, d MMMM"), "\n";

echo $d->plus(1, Core\Unit::Month)->format("yyyy-MM-dd"), "\n";
echo $d->minus(1, Core\Unit::Day)->format("EEE d MMM"), "\n";
echo $d->with({month: 3})->format("yyyy-MM-dd"), "\n";
echo $d->compareTo(Core\Time\Date::at(2024, 2, 1)), "\n";

try {
    Core\Time\Date::at(2023, 2, 29);
} catch (RuntimeError $bad) {
    echo "2023-02-29 does not exist\n";
}

var $utc = Core\Time\Zone::UTC;
echo Core\Time::at(2024, 3, 1, $utc, {hour: 23})->date()->format("d/M/yyyy"), "\n";
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
```
