- **`nvs-cli`'s `a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names` asserts a
  ratio, so a loaded machine fails it from either side.** Each half takes the fastest of five
  attempts, but the halves do not run at the same moment, so under load warm can come out *slower*
  than cold — an inversion no cache regression produces, since a real one narrows the gap toward 1x
  rather than crossing it. `cargo test --bin nvs cache::tests::a_warm_start` alone
  settles it in a second.
  [until: gone crates/nvs-cli/src/cache.rs:a_warm_start_is_faster_than_a_cold_one]
