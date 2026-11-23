`nvs test <paths>` runs tests. The path decides which kind of test runs.

A `.nvs` file is a program, and the command runs every method that has the `#[Test]` attribute. A
directory with `.nvs` files is run as one program. Any other path is a `.nvst` case file, or a
directory of such files. A case is one program with its expected output.

`nvs test CartTest.nvs:42` runs the test that contains line 42. `--filter <text>` runs the tests
whose name contains the text. `--tag` and `--skip-tag` select tests by the tags that
`#[Test(tags: [...])]` gives them. `--since main` runs the tests that, in an earlier run, ran code
you changed or deleted since the git branch `main`. When it cannot tell which tests a change
affects, it runs every test.

`--fail-fast` stops after the first failure. `--rerun-failed` runs the tests that failed in the
last run. `--watch` runs the tests again each time you save a file. `nvs test --help` lists every
flag.

The exit status is not zero when a test failed. A skipped test is not a failure.

**Good to know:** a `Core\Test::assertMatchesInline` snapshot is the expected text, written in the
test's source file. `--update` writes the new text into each failed snapshot. Those tests are still
reported as failed, so run the tests again.
