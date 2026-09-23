- **`a_revalidation_that_wins_publishes_and_readers_never_block_on_a_compile` fails under a full
  `verify.py` and passes on its own.** It asserts how many reads land while a compile is running,
  which is a timing claim, and `test` runs its binaries side by side — a loaded machine lets the
  compile finish between the two reads and the count comes back one short. Re-run `python
  tools/verify.py` before believing it: the site is `crates/nvs-cli/src/script.rs:1709` and it is
  nothing a stdlib or docs session touched.
  [until: gone crates/nvs-cli/src/script.rs:a reader was answered once and then waited the compile out]
