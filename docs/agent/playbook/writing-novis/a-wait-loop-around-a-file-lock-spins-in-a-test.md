- **A wait loop around `Core\IO\File::lock` spins in a test, because `Core\Time::sleep` returns at
  once under the test's virtual clock.** `lock` never waits, so retrying it with a sleep gives up
  after its tries in microseconds whenever two parallel tests meet. Serialize with the database's
  write lock instead, whose wait is real time, as `apps/registry/src/Index.nvs`'s `catchUp` does.
  `until: gone apps/registry/src/Index.nvs:Other requests that write wait`
