---
summary: the clock — the current `Instant`, a monotonic reading to time code, and `sleep`
keywords: time, current time, now, clock, timestamp, monotonic, elapsed time, measure time, sleep, wait, pause
---

`Core\Time` is the clock. `now()` returns the current time as an `Instant`. `monotonic()` returns a
reading that only moves forward, to measure how long code takes. `sleep($duration)` waits for a
`Duration`, such as `50ms`.

Each time class builds its own values. `Instant::fromIso` and `Instant::fromEpoch` build an
`Instant`. `Date::fromIso` and `Date::at` build a `Date`, and `TimeOfDay::fromIso` and
`TimeOfDay::at` build a `TimeOfDay`. `DateTime::fromIso`, `DateTime::at` and `DateTime::parse` build a
`DateTime` in a zone.

There is no default time zone. An `Instant` becomes a date and a time of day only with
`->in($zone)`. A zone that is fixed in the code is written as text, such as `'Europe/Berlin'`.

```nvs
<?nvs
var $now = Core\Time::now();
echo $now->toEpochSeconds() > 1704067200 ? 'after 2024' : 'before 2024', "\n";

var $start = Core\Time::monotonic();
Core\Time::sleep(20ms);
Core\Time\Duration $took = Core\Time::monotonic()->minus($start);
echo $took->toMilliseconds() >= 20 ? 'waited at least 20ms' : 'too short', "\n";

var $meeting = Core\Time\DateTime::fromIso('2024-03-01 09:30', 'Europe/Berlin');
echo $meeting->toInstant()->toIso(), "\n";
```
```output
after 2024
waited at least 20ms
2024-03-01T08:30:00Z
```
