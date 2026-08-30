---
summary: an absolute point on the timeline, to the nanosecond, with no zone
keywords: getTimestamp, date("U"), microtime(true), date_add, date_sub, modify, date_diff, DateInterval, DATE_ATOM, timestamp, epoch, timeline, UTC
---

An `Instant` is a point on the timeline with no calendar attached: it has an epoch count and an
RFC 3339 rendering in UTC, and nothing else until `->in($zone)` places it on a calendar. It moves
only by an exact `Duration` — `plus(36h)` is thirty-six hours whatever a zone's clocks did in between —
and `since` measures the exact gap between two instants; a step in days or months is taken on the
`DateTime` that `->in($zone)` answers. Two instants are ordered with `compareTo`; the `<`, `>` and
`<=>` operators do not compile on an `Instant`.

```nvs
<?nvs
var $start = Core\Time::fromIso("2024-03-01T12:00:00Z");
echo $start->toIso(), "\n";
echo $start->toEpochSeconds(), "\n";
echo $start->toEpochMillis(), "\n";

var $end = $start->plus(36h);
echo $end->toIso(), "\n";
echo $end->since($start), "\n";
echo $start->minus(30m)->toIso(), "\n";
echo $start->compareTo($end), "\n";

echo $start->in(Core\Time\Zone::of("Asia/Tokyo"))->format("yyyy-MM-dd HH:mm"), "\n";
echo Core\Time::fromEpoch(1709294400, {nanos: 500000000})->toIso(), "\n";
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
