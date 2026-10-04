---
summary: an exact, signed span of nanoseconds — a duration written like `1h30m` is a value of this class
keywords: DateInterval, sleep, usleep, hrtime, strtotime("+3 days"), 1h30m, 250ms, nanoseconds, exact offset, timeout
---

A `Duration` is a signed count of nanoseconds. You write one directly in the code as a number and a
unit — `30s`, `250ms`, `1h30m`, `7d` — with the units `ns`, `us`, `ms`, `s`, `m`, `h`, `d` and `w`, and
`Duration::parse` reads the same grammar from text at run time. It is exact: `days(1)` and `1d` are
twenty-four hours, never a calendar day, which is what `Core\Time\DateTime::plus($n, Core\Unit::Day)`
is. `echo` renders a duration back in that same syntax, so its text round-trips through `parse`. An
argument to `parse` written directly in the code is read when the program compiles, so a bad one is a compile error; text
computed at run time throws a `ParseError`. Two durations are ordered with `compareTo`; the `<`
operators do not compile on a `Duration`.

```nvs
<?nvs
var $d = 1h30m;
echo $d, "\n";
echo $d->toSeconds(), "\n";

echo $d->plus(15m), "\n";
echo $d->minus(2h), "\n";
echo $d->multipliedBy(2), "\n";
echo $d->negated(), "\n";

echo Core\Time\Duration::parse("250ms")->toMilliseconds(), "\n";
echo Core\Time\Duration::minutes(90)->compareTo($d), "\n";
echo Core\Time\Duration::seconds(90061), "\n";
echo 1500ms->toString(), "\n";

string $flag = "--since=30 seconds";
try {
    Core\Time\Duration::parse(Core\Str::slice($flag, 8));
} catch (ParseError $bad) {
    echo "not a duration\n";
}
```
```output
1h30m
5400
1h45m
-30m
3h
-1h30m
250
0
1d1h1m1s
1s500ms
not a duration
```
