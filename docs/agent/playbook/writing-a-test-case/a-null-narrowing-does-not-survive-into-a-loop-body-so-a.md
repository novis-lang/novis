- **A `!= null` narrowing does not survive into a loop body, so a nullable receiver is nullable
  again inside a `foreach`.** `if ($found != null) { … }` narrows, and `$found->group($key)` inside
  a `foreach` four lines below is `E0459` ("this receiver is nullable"); passing `$found` to a
  helper declaring `Core\Regex\Match` is `E0401: found null|Core\Regex\Match` from inside the loop
  and accepted outside it. Declare the helper's parameter `?Core\Regex\Match` and read it with `?->`
  plus `??`, rather than narrowing once at the top and trusting it. [until: reviewed 2026-09-06]
