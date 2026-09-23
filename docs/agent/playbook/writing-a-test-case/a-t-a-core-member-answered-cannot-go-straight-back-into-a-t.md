- **A `?T` a `Core` member answered cannot go straight back into a `T` parameter, and the diagnostic
  lands at the argument.** `?int $step = Core\Totp::check($code, $secret);` then
  `Core\Totp::check($code, $secret, $step)` is `error[E0401]: expected int, found int|null` pointing
  at `$step`, which reads like the member's row is wrong. Put the second call inside the `else` of
  `if ($step == null)`, where the type is narrowed; every case that round-trips a `?T` answer pays
  one `if`. [until: reviewed 2026-09-06]
