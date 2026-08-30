---
summary: the clock and the constructors — an absolute `Instant`, or a civil `DateTime` built in a named `Zone`
keywords: time, microtime, date_create, hrtime, sleep, usleep, time_nanosleep, time_sleep_until, DateTime::setTimestamp, strtotime, DateTime::__construct, DateTime::createFromFormat, strptime, mktime, gmmktime, DateTime::setDate, DateTimeImmutable, checkdate, date_default_timezone_set, clock, timestamp
---

`Core\Time` is where a time value starts: `now()` reads the clock as an `Instant`, `fromIso` and
`fromEpoch` build one from a timestamp, and `at` and `parse` build a `DateTime` from civil fields or
text. Every value it answers is immutable, and there is no ambient time zone — a `DateTime` is always
built in a `Zone` the call names, and an `Instant` becomes a calendar reading only through
`->in($zone)`. The two arithmetics are told apart by type: an `Instant` moves by an exact `Duration`
(`->plus(24h)`), a `DateTime` moves by a calendar step (`->plus(1, Core\Unit::Day)`), and across a
DST boundary the two land an hour apart. `strtotime`'s free-form text does not exist; each of its
phrases is a typed call, and a run-time offset such as `"30d"` is `Core\Time\Duration::parse`.

```nvs
<?nvs
var $utc = Core\Time\Zone::UTC;
var $meeting = Core\Time::at(2024, 3, 1, $utc, {hour: 9, minute: 30});
echo $meeting->format("yyyy-MM-dd HH:mm"), "\n";

var $parsed = Core\Time::parse("5 March 2024", "d MMMM yyyy", $utc);
echo $parsed->format("yyyy-MM-dd HH:mm:ss"), "\n";

var $stamp = Core\Time::fromIso("2024-03-01T10:30:00+01:00");
echo $stamp->toIso(), "\n";
echo $stamp->compareTo($meeting->toInstant()) == 0 ? "same instant" : "different", "\n";

echo Core\Time::fromEpoch(0)->in($utc)->format("EEEE, d MMMM yyyy"), "\n";

var $berlin = Core\Time\Zone::of("Europe/Berlin");
echo $meeting->toInstant()->in($berlin)->format("HH:mm VV"), "\n";

var $eve = Core\Time::at(2024, 3, 30, $berlin, {hour: 12});
echo $eve->plus(1, Core\Unit::Day)->format("yyyy-MM-dd HH:mm"), "\n";
echo $eve->toInstant()->plus(24h)->in($berlin)->format("yyyy-MM-dd HH:mm"), "\n";

try {
    Core\Time::at(2024, 2, 30, $utc);
} catch (RuntimeError $bad) {
    echo "30 February does not exist\n";
}
```
```output
2024-03-01 09:30
2024-03-05 00:00:00
2024-03-01T09:30:00Z
same instant
Thursday, 1 January 1970
10:30 Europe/Berlin
2024-03-31 12:00
2024-03-31 13:00
30 February does not exist
```
