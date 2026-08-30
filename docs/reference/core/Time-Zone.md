---
summary: a time zone — an IANA region, a fixed offset, or `UTC` — named explicitly at every conversion
keywords: DateTimeZone, date_default_timezone_get, date_default_timezone_set, getOffset, timezone, IANA, Europe/Berlin, UTC, offset, DST
---

A `Zone` is what turns an `Instant` into a calendar reading and back: `Zone::of` looks up an IANA
identifier such as `Europe/Berlin` in the bundled database and throws on one it does not have,
`Zone::fixed` is a constant offset with no DST rules, and `Zone::UTC` is the one constant. There is
no ambient zone: `Zone::system()` answers the host's zone as an ordinary value the program then
passes on, and nothing like `date_default_timezone_set` exists. A zone with DST has no single
offset, so `offsetAt` asks for the instant.

```nvs
<?nvs
var $berlin = Core\Time\Zone::of("Europe/Berlin");
var $jan = Core\Time::fromIso("2024-01-15T12:00:00Z");
var $jul = Core\Time::fromIso("2024-07-15T12:00:00Z");
echo $berlin->offsetAt($jan), "\n";
echo $berlin->offsetAt($jul), "\n";
echo $jul->in($berlin)->format("HH:mm xxx VV"), "\n";

var $india = Core\Time\Zone::fixed(5h30m);
echo $jul->in($india)->format("HH:mm xxx"), "\n";
echo Core\Time\Zone::UTC->offsetAt($jul)->toSeconds(), "\n";

echo Core\Time::at(2024, 7, 15, $berlin, {hour: 14})->toInstant()->toIso(), "\n";

try {
    Core\Time\Zone::of("Mars/Olympus");
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
