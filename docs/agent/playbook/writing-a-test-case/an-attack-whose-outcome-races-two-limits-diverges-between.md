- **An attack whose outcome races two limits diverges between the judged and the recording run.**
  `bun nv proofs` judges on the release binary and records on the slower covws one, so an endless
  allocation hit the memory limit on one and the wall limit on the other, and the run printed
  `diverged`. Make one limit win on every build, such as a long `#[Test(timeout: ...)]` beside a fast
  allocation, as `tests/hostile/tools/cli/nvs-test-mutate/01-a-mutant-that-never-ends.nvs` does.
  `until: gone tests/hostile/tools/cli/nvs-test-mutate/01-a-mutant-that-never-ends.nvs`
