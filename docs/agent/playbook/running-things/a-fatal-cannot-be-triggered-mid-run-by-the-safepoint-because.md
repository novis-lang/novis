- **A `FATAL` cannot be triggered mid-run by the safepoint, because the flag can only be set before
  the run and the script frame's own entry poll fires first** — which is why
  `a_fatal_is_never_caught` sees empty output. The stack limit is no better: `arm_stack_limit`'s
  soft tier answers a catchable `Recursion` long before the floor. What is reachable from source
  after locals are live is a `Fault::fatal` from `nvs-stdlib` — `Core\Arr::countBy` over an
  `array<float>` is one — so reach for that when a test needs a fatal at a chosen point.
  [until: reviewed 2026-09-06]
