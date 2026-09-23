- **A `cargo-named` floor check can go red on a *timing* test that the driver's own load broke, and
  it reads exactly like a regression the last commit caused.** `cache::tests::a_warm_start_is_faster
  _than_a_cold_one_by_the_margin_this_test_names` failed once after a session that touched only
  `nvs-stdlib`, because the guard took each arm's fastest sample over the whole sweep and a release
  prebuild running beside it stalled every warm arm. Run the named test by itself before believing a
  perf guard's failure — it passes at 10x on an idle box — and if it is load-sensitive, fix the
  statistic rather than the margin. [until: reviewed 2026-10-13]
