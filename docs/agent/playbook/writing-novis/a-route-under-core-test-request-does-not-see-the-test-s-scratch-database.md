- **A route under `Core\Test::request` runs in an isolate of its own, so it does not see the rows the
  test wrote to a `:memory:` database, the test's statics, or `Core\Env::Context::Test`.** A probe
  put a row in `scratch` and the route counted `0`, and the context read `Request`. To test
  what a page shows, build it from a function that takes its data, such as `Pages::home(Totals, ...)`,
  on the test's own connection. Send a request only to check the status, the headers and the cookies,
  as `apps/telemetry/src/PagesTest.nvs` does. A header from `[http.headers]` is not in the test's
  response either: set it from the gate.
  `until: gone apps/telemetry/src/PagesTest.nvs:Pages::home`
