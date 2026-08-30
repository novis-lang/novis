---
summary: a wall-clock reading with no date and no zone, wrapping within the day
keywords: setTime, date("H:i"), wall clock, time of day, clock reading, hour, minute
---

A `TimeOfDay` is a clock reading — hour, minute, second, nanosecond — with no date and no zone, built by
`TimeOfDay::at` or read off a `DateTime` with `->timeOfDay()`. It steps by `Core\Unit::Hour` and
smaller and wraps within the day, so `23:30` plus an hour is `00:30`; `Core\Unit::Day` and larger are
refused, because a carry has no date to go to. `format` takes the time letters of the CLDR grammar
`DateTime::format` uses, and a date or zone letter is refused. `DateTime::withTime` puts a reading
back onto a date. Two readings are ordered with `compareTo`; the `<` operators do not compile.

```nvs
<?nvs
var $t = Core\Time\TimeOfDay::at(23, 30);
echo $t->format("HH:mm:ss"), "\n";
echo $t->format("h:mm a"), "\n";

echo $t->plus(1, Core\Unit::Hour)->format("HH:mm"), "\n";
echo $t->minus(45, Core\Unit::Minute)->format("HH:mm"), "\n";
echo $t->with({second: 45})->format("HH:mm:ss"), "\n";
echo $t->compareTo(Core\Time\TimeOfDay::at(9, 0)), "\n";

var $utc = Core\Time\Zone::UTC;
var $d = Core\Time::at(2024, 3, 1, $utc, {hour: 9});
echo $d->withTime($t)->format("yyyy-MM-dd HH:mm"), "\n";
echo $d->timeOfDay()->format("HH:mm"), "\n";
```
```output
23:30:00
11:30 PM
00:30
22:45
23:30:45
1
2024-03-01 23:30
09:00
```
