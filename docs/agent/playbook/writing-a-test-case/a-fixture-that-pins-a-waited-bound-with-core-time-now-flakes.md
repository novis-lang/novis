- **A fixture that pins a *waited* bound with `Core\Time::now` flakes on the WSL leg, and the
  failure accuses the code it is pinning.** `examples/pool.nvs` read the wall clock across a
  500ms pool `acquire` whose deadline is a monotonic `Instant`, so a realtime step under the
  wait — which WSL2 takes — reported 464ms and printed a verdict accusing the pool of refusing
  early. Measure an elapsed interval with `Core\Time::monotonic()->minus($mark)->toMilliseconds()`,
  and read `rule:http-server/every-deadline-is-monotonic` for why the two clocks are not
  interchangeable. [until: reviewed 2026-09-11]
