- **Check a `gaps.py --errors` site's own parameter types before taking it as catchable.** A
  `Fault::thrown` guarding an *element* of a typed parameter may be unreachable:
  `Core\Csv::format`'s "column N holds a value that is not a `string`" sits behind an
  `array<array<string>>` parameter, and every way round it is `E0401` or the `array<T> as array<U>`
  cast that panics `nvs-ir`. An entry is a candidate, not a plan; four `nvs run` calls on a scratch
  file judge one. [until: reviewed 2026-09-06]
