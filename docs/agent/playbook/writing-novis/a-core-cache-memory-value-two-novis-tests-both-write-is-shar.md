- **A `Core\Cache::memory()` value two Novis tests both write is shared and can change under either,
  because `nvs test` runs tests in parallel in one process.** Two tests on the same `at:` day each
  made `Limit`'s daily salt, and one test's key changed between two calls. Give each test that writes
  a cache entry its own key or day, as `apps/telemetry/src/LimitTest.nvs` does. `Core\Test::request`
  also sends no client address: `Core\Request::clientIp()` is `null` in it.
  `until: gone apps/telemetry/src/LimitTest.nvs:its own day`
