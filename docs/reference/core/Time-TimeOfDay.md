---
summary: a wall-clock reading with no date and no zone, wrapping within the day
keywords: wall clock, time of day, time without a date, clock reading, parse a time, ISO 8601, hour, minute, second
---

A `TimeOfDay` is a clock reading: hour, minute, second and nanosecond. It has no date and no zone.
`TimeOfDay::fromIso` reads a time written as `HH:MM`, `HH:MM:SS` or `HH:MM:SS.f`, such as `09:30`.
`TimeOfDay::at` builds one from numbers, and `->timeOfDay()` gives the time of a `DateTime`. A time
written as a string literal in `TimeOfDay::fromIso` is checked when the program compiles.

`plus` and `minus` step by `Core\Unit::Hour` or a smaller unit, and wrap within the day: `23:30`
plus one hour is `00:30`. `Core\Unit::Day` and larger units throw an error, because there is no date
to change. `format` takes the time letters of the patterns `DateTime::format` uses. A date letter or
a zone letter does not compile. `DateTime::withTime` puts a time onto a date. `<`, `>`, `<=`, `>=`
and `compareTo` order two times.

`hour()`, `minute()`, `second()` and `nanos()` return the four fields as numbers. Each returns a
`uint`, with the same name as the field `with` takes.

```nvs
<?nvs
var $t = Core\Time\TimeOfDay::fromIso('23:30');
echo $t->format('HH:mm:ss'), "\n";
echo $t->format('h:mm a'), "\n";

echo $t->plus(1, Core\Unit::Hour)->format('HH:mm'), "\n";
echo $t->minus(45, Core\Unit::Minute)->format('HH:mm'), "\n";
echo $t->with({ second: 45 })->format('HH:mm:ss'), "\n";
echo $t->compareTo(Core\Time\TimeOfDay::at(9, 0)), "\n";

var $d = Core\Time\DateTime::fromIso('2024-03-01 09:00', 'UTC');
echo $d->withTime($t)->format('yyyy-MM-dd HH:mm'), "\n";
echo $d->timeOfDay()->format('HH:mm'), "\n";
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
