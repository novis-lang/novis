- **A `Core\Fatal::onLimit` handler runs only when the breach lands inside the reserve, so a large
  overshoot prints nothing.** `Ctx::run_limit_handler` adds the reserve back to the reduced ceiling,
  so a request already holding more than `[limits] memory` breaches again at the handler's first
  helper call (`echo` is one) and is abandoned without a word; the `FATAL` message is identical
  either way. Size the ballast to land between `memory` minus the reserve and `memory`.
  [until: reviewed 2026-09-06]
