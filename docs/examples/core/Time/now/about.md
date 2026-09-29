Reads the current time from the system clock and returns it as an `Instant`. An `Instant` is one
moment, the same everywhere in the world. It replaces PHP's `time`, `microtime` and `date_create`.

An `Instant` has no time zone. To get a date and a time of day, call `->in($zone)` with the zone
you want. You can store the moment with `toIso` or `toEpochSeconds` and read it back later. To
measure how long something takes, use `Core\Time::monotonic` instead. The system clock can be
changed while a program runs, and `monotonic` cannot.

In a test, `#[Test(at: ...)]` sets a fixed time, and `now` returns that time.

**The examples below** show the current date in a zone, two readings one after the other, and a
login session that expires after 30 minutes.
