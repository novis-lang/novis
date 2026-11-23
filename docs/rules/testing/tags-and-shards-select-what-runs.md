`#[Test(tags: ["slow", "db"])]` labels a test, `--tag` and `--skip-tag` select by label,
`--shard N/M` runs the Nth of M disjoint parts of the selection, `--since <ref>` runs the tests a
change since a git ref can reach, and `<file>:<line>` runs the test written at that line.

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

**`<file>:<line>` selects by where a test is written.** A path argument that does not exist, ends in
`:` and digits, and names an existing file before that colon is read as a file and a line, so
`C:\app\cart_test.nvs:42` keeps its drive letter. A line in a test method, its attributes included,
runs that test with every data row. A line in a test class outside its tests runs the whole class. A
line outside every test class exits with status 2 and names the nearest test class in the file. Only
one path may name a line, and a `.nvst` tree refuses it.

**`--since <ref>` runs the tests a change can reach.** It reads the changed lines from git as
`rule:testing/mutation-testing` does. A run that measures per-test coverage writes a *reached map*
(`rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle` says where): each test with the
methods it ran, and a hash of each method body, of each test class outside its bodies, and of each
program file outside its test classes and bodies. A statement in an anonymous function or class
counts for the method it is written in. A coverage flag, `--mutate` and `--since` write the map, so a
`--since` run has its coverage probes on. A run with none of them writes nothing and pays nothing.

`--since` runs more tests when it cannot tell, never fewer. A method is changed when a changed line
is in its body after the line of its `{`, when its body's hash differs from the map's, or when the map
has it and the program does not, as for a deleted file or method. A rename is read as a delete and an
add (`git diff --no-renames`). A test runs when it reached a changed method, when a line of its own
method changed, or when the map does not name it. A changed line in a test class outside its tests,
or a class hash that differs, runs every test of that class. **Every selected test runs**, and the
run prints one line saying why, when:

- there is no map for this entry;
- a changed line that is not blank is outside every method body and test class: a class header, a
  property, a constant, a property hook, a method's first line, an attribute or a `use`. A new
  override changes which code a call reaches, on lines no test reached before;
- a program file is not in the map, or its hash differs;
- a file in the program's folder that is not a `.nvs` file changed or was deleted, such as `nvs.toml`
  or a data file a test reads;
- a file outside the program's folder that is not a program file changed, or a changed path could
  not be resolved, such as a deleted file whose folder is gone.

A run that left a test out keeps that test's entry, and the stored hash of each method, class and
file it depends on, so a change it did not run against still selects it at the next run. A method
reached both by a test that ran and by one left out keeps the old hash too, so the test that ran runs
once more. That costs at most one extra run of a test, and the map agrees with the program after the
next `--since` run. A directory of programs has the folder it is in as its program folder, so more
paths count as the program's own, which runs more tests, never fewer.

A line that was only deleted sits between two lines, and is in a body only when both are. The hashes
keep the map honest when it was written from a working tree that has changed since: an edit put back
the way the ref has it is not in git's diff, and its body hash still differs. The file and class
hashes read each run of whitespace as one space, so a blank line between classes changes neither. A
data file edited and put back since the map was written is not seen. `--since` is for the
edit-and-run loop, and CI runs the whole suite. Under `--mutate`, `--since` narrows the mutants and
every test runs, because a mutant runs the tests coverage saw reach it.

Every selector combines with the others, and a test must pass all of them. `--since` beside
`--rerun-failed` is an error, because the two choose tests by different evidence.

- **Memory:** nothing per test beyond its list of tags, held in the test table while compiling. A run
  that writes or reads the reached map holds one label and one line range per method body and per
  test class, and one label set per test, for the length of the run.

**Declined:**
- **Regular expressions or globs in `--filter`.** A substring, tags and shards already select every
  set a suite has needed, and each syntax is one more thing to learn.
- **A coverage minimum flag.** CI reads the coverage files, and a threshold there is one line of CI.
- **A slowest-tests flag.** `--format json` has `durationMs` per test.
- **A hash of each whole file in the map.** After any edit, every method of the file would count as
  changed, and `--since` would select by file.
- **Shards balanced by recorded run time.** It needs a timing file every machine reads, and a part
  would change whenever the timings do, so two runs of one commit could split differently.
- **Shards by position in declaration order.** Adding one test would move every test after it to
  another part.
- **A closed list of tags declared somewhere.** A typo in a tag selects nothing, and the summary's count
  shows it. A roster would be a second place to edit for every new label.
