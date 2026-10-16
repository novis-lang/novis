`nvs test --watch` runs the selected tests, then runs them again each time a file the program
requires changes, until it is interrupted.

The change is found by the same background check that brings an edit to a running `nvs run` or
`nvs serve` (`rule:config/an-edit-reaches-the-next-request-without-a-restart`), so `--watch` adds no
second file watcher and sees exactly the files a compile reads. Every pass recompiles what changed and
runs the whole selection: `--filter`, the tag flags and `--rerun-failed` apply to each pass. A pass
that does not compile prints its diagnostics and waits for the next change. A change that arrives while
a pass runs starts the next pass when this one ends, never in the middle of it.

`--watch` runs a program's `#[Test]` methods. It is refused with `--list` and `--update`, which do one
thing and stop, and with a machine format, whose one document per run a consumer reads once. A `.nvst`
tree is not a program and has no require graph to watch, so naming one with `--watch` is refused.

- **Memory:** the compiled program between passes, which the background check already holds for
  `nvs run`. Each pass's tests are released when the pass is reported.

**Not on disk.** `nvs test` has no `--watch` (`crates/nvs-cli/src/main.rs:574`). The background check
is `crates/nvs-cli/src/script.rs:1501`.

**Declined:**
- **Running only the tests a change reaches.** It needs a map from every line to the tests that reach
  it, which only a coverage run builds, and a wrong map skips a test that should have failed.
- **A watcher of its own over the project folder.** It would see files the program never reads, and
  miss none the background check misses.
