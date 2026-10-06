A request reads its own memory as three members on `Core\Budget` — `memoryHeld()` is the bytes it
holds right now, `memoryPeak()` the high-water mark of that figure for this request, and
`memoryLimit()` the ceiling both are measured against.

```nvs
Core\Budget::memoryHeld():  int
Core\Budget::memoryPeak():  int
Core\Budget::memoryLimit(): int
```

**`held`, not `usage`**, because the runtime already says *held*: a breach renders as *"the request
exceeded its memory limit — N bytes held against a ceiling of M"*, and a member whose name disagrees
with the error text about the same quantity is a second vocabulary to learn. `usage` is also the word
that carries an ambiguity between "occupied now" and "consumed in total", and only one of those is
ever meant.

**`memoryLimit` is among them because a peak with no scale is not actionable.** The ceiling is
otherwise reachable only as `Core\Config::get('limits.memory')` — a string with a suffix that every
call site would parse — so the question the trio exists to answer stays one expression:

```nvs
if (Core\Budget::memoryPeak() * 10 > Core\Budget::memoryLimit() * 9) { … }
```

An uncapped request — `[limits.hard] memory = false`, which
`rule:config/three-changeability-classes` permits — answers `0` from `memoryLimit`, the same reading
of zero as "no ceiling" the runtime's own limit check already uses, rather than a second spelling for
it.

**The process's memory is `Core\Os`'s and is a different question.** `Core\Os::residentBytes()` is
the resident set — what `getrusage`'s `ru_maxrss` answers — beside `pid`, `hostname`, `cpuCount` and
`loadAverage`, which are host and process facts too. Two classes, two names, and neither readable as
the other: a per-request figure sitting among host facts would be read as process memory by everyone
who had not been told otherwise, which is the same ambiguity relocated rather than removed.

There is no `$real_usage`-style boolean in any spelling. Two accountings behind one member is what
`rule:core-api/no-mode-strings` refuses, and where two numbers are genuinely different questions they
are two members on the two classes that own them.
