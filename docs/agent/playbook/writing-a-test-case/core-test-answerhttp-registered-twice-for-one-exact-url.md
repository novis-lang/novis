- **`Core\Test::answerHttp` registered twice for one exact URL serves the first answer every time,
  so a second registration does not script a reconnect.** Answers accumulate and the first exact
  match wins, so an example that reads a stream, "connects again" and expects new events prints the
  first body twice. Give the second connection its own URL (a query such as `?after=2` is the
  honest one), and read `nvs agent show 'Core\Test::answerHttp'` for the matching order.
  [until: reviewed 2026-09-23]
