---
summary: a time zone — an IANA region, a fixed offset, or `UTC` — named explicitly at every conversion
keywords: timezone, time zone, IANA, tz database, Europe/Berlin, UTC, offset, UTC offset, DST, daylight saving time, convert time zone, system time zone
---

A `Zone` turns an `Instant` into a date and a time of day, and back. A zone that is fixed in the code
is written as text where a `Zone` is expected: `$instant->in('Europe/Berlin')`. The name is checked
when the program compiles, and the zone is built once. `UTC` and an offset such as `'+05:30'` are
written the same way.

`Zone::of($name)` looks up a name that arrives while the program runs, such as a user's setting. It
throws an error for a name the time zone database does not have. `Zone::fixed` is a constant offset
with no daylight saving rules. There is no default zone: `Zone::system()` returns the zone of the
host, and the program passes it on. A zone with daylight saving time has no single offset, so
`offsetAt` needs the instant.

```nvs
<?nvs
var $berlin = Core\Time\Zone::of('Europe/Berlin');
var $jan = Core\Time\Instant::fromIso('2024-01-15T12:00:00Z');
var $jul = Core\Time\Instant::fromIso('2024-07-15T12:00:00Z');
echo $berlin->offsetAt($jan), "\n";
echo $berlin->offsetAt($jul), "\n";
echo $jul->in('Europe/Berlin')->format('HH:mm xxx VV'), "\n";

var $india = Core\Time\Zone::fixed(5h30m);
echo $jul->in($india)->format('HH:mm xxx'), "\n";
echo Core\Time\Zone::UTC->offsetAt($jul)->toSeconds(), "\n";

echo Core\Time\DateTime::fromIso('2024-07-15 14:00', $berlin)->toInstant()->toIso(), "\n";

string $saved = 'Mars/Olympus';
try {
    Core\Time\Zone::of($saved);
} catch (RuntimeError $unknown) {
    echo "not in the database\n";
}
```
```output
1h
2h
14:00 +02:00 Europe/Berlin
17:30 +05:30
0
2024-07-15T12:00:00Z
not in the database
```
