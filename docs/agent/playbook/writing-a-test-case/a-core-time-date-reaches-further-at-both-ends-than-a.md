- **A `Core\Time\Date` reaches further at both ends than a `DateTime`, so a range-ends sweep builds
  the two halves differently.** `Core\Time\Date::at(9999, 12, 31)` and `at(-9999, 1, 1)` pass while
  `Core\Time::at(9999, 12, 31, $utc)` and `at(-9999, 1, 2, $utc)` throw, because jiff's timestamp
  range stops inside the calendar's last day and first two; the refusal says the conversion
  overflowed. Read a year's length from March rather than from either end.
  [until: reviewed 2026-09-06]
