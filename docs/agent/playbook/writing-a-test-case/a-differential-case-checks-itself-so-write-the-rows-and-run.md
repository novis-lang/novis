- **A differential case checks itself, so write the rows and run it rather than pricing PHP's answer
  by hand first.** An `--ORACLE--` case's failure output prints both columns side by side, which is
  the whole comparison in one call. Reach for `php -r` on the *divergence* half instead, where the
  frozen `--EXPECT--` is Novis's own output and PHP's answer only appears in the case's prose — the
  one sentence the runner cannot check. [until: reviewed 2026-09-06]
