- **A storage resolved at compile time and a spelling PHP resolves at run time agree until two
  classes disagree, so a scratch file judging one has to make them disagree.** `static::$total`
  inside `Base` answered `Base`'s slot for `Sub::viaStatic()` where PHP answers `Sub`'s, and only a
  subclass that *redeclares* the static shows it (it is `E0499` now rather than a silent
  difference). Write the redeclaring subclass into the probe before trusting a match with PHP.
  [until: reviewed 2026-09-06]
