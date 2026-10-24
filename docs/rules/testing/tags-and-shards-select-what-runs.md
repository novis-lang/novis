`#[Test(tags: ["slow", "db"])]` labels a test, `--tag` and `--skip-tag` select by label, and
`--shard N/M` runs the Nth of M disjoint parts of the selection.

A tag is a string, and a test may carry any number of them. `--tag slow` runs only the tests that carry
`slow`, and `--skip-tag slow` leaves them out. Both flags may be repeated: a test runs when it carries
at least one `--tag` label, or when no `--tag` is given, and when it carries no `--skip-tag` label. They
combine with `--filter`, and a test must pass every one. The compiler checks that each tag is a
non-empty string literal, and nothing more: the set of tags is the program's own. A `.nvst` case has no
tags, so `--tag` and `--skip-tag` beside a case tree are an error.

`--shard 2/4` splits the selected tests into four parts and runs the second. A test's part is a hash of
its `Class::method` name, a data row going with its test, so the parts do not change when another test
is added or removed, and four machines that each run one part run every selected test exactly once. A
`.nvst` case's part is a hash of its path. The parts are disjoint and together they are the whole
selection. They are not balanced by run time.

The summary says how many tests the selection left out, so a run that selected nothing is visible as
one. A `.nvst` run writes that count on its own line above the summary line, which stays the last.
The hash is FNV-1a over the name's bytes, with `/` between a path's parts on every platform, so every
machine of one CI run computes the same parts (`crates/nvs-test/src/shard.rs`).

- **Memory:** nothing per test beyond its list of tags, held in the test table while compiling.

**Declined:**
- **Shards balanced by recorded run time.** It needs a timing file every machine reads, and a part
  would change whenever the timings do, so two runs of one commit could split differently.
- **Shards by position in declaration order.** Adding one test would move every test after it to
  another part.
- **A closed list of tags declared somewhere.** A typo in a tag selects nothing, and the summary's count
  shows it. A roster would be a second place to edit for every new label.
