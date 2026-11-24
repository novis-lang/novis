---
summary: an absolute point on the timeline, to the nanosecond, with no zone
keywords: instant, timestamp, epoch, Unix time, milliseconds, timeline, UTC, RFC 3339, ISO 8601, from timestamp, parse a timestamp, time difference, add hours
---

An `Instant` is a point on the timeline. It has no calendar and no zone. `Instant::fromIso` reads an
RFC 3339 timestamp such as `2024-03-01T12:00:00Z`. `Instant::fromEpoch` builds one from seconds
since 1 January 1970. `Core\Time::now()` returns the current instant.

An `Instant` has an epoch count and a text form in UTC. `->in($zone)` gives its date and time of day
in a zone, as a `DateTime`. It moves only by an exact `Duration`: `plus(36h)` is always thirty-six
hours. `since` returns the exact time between two instants. A step in days or months is taken on the
`DateTime` that `->in($zone)` returns. `compareTo` orders two instants. The `<`, `>` and `<=>`
operators do not compile on an `Instant`.

```nvs
<?nvs
var $start = Core\Time\Instant::fromIso('2024-03-01T12:00:00Z');
echo $start->toIso(), "\n";
echo $start->toEpochSeconds(), "\n";
echo $start->toEpochMillis(), "\n";

var $end = $start->plus(36h);
echo $end->toIso(), "\n";
echo $end->since($start), "\n";
echo $start->minus(30m)->toIso(), "\n";
echo $start->compareTo($end), "\n";

echo $start->in('Asia/Tokyo')->format('yyyy-MM-dd HH:mm'), "\n";
echo Core\Time\Instant::fromEpoch(1709294400, { nanoseconds: 500000000 })->toIso(), "\n";
```
```output
2024-03-01T12:00:00Z
1709294400
1709294400000
2024-03-03T00:00:00Z
1d12h
2024-03-01T11:30:00Z
-1
2024-03-01 21:00
2024-03-01T12:00:00.5Z
```
