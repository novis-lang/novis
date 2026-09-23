- **An `--ORACLE--` helper named after a PHP built-in is a fatal, and the runner reports it as a
  *case* failure.** A `function pos($v)` in the oracle half is `Cannot redeclare function pos()`,
  because `pos()` is `current()`'s alias and PHP has ~1,900 globals; the runner prints `PHP exited
  255` plus the stderr. Prefix every oracle helper with `php` (`phpAfter`, `phpLines`, `phpSort`) as
  the `str-before-and-after` and `arr-*` cases do, and keep the un-prefixed spellings for the Novis
  side. [until: reviewed 2026-09-06]
