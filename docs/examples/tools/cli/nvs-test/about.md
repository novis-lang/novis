`nvs test <paths>` runs tests. A `.nvs` file is a program, and the command runs every method that
has the `#[Test]` attribute. A directory with `.nvs` files runs as one program. Any other path is a
`.nvst` case file, or a directory of such files. A case is one program with its expected output.

`nvs test CartTest.nvs:42` runs the test that contains line 42. `--filter <text>` runs the tests
whose name contains the text. `--tag` and `--skip-tag` select tests by their
`#[Test(tags: [...])]`. `--since main` runs the tests that, in an earlier run, ran code you changed
or deleted since the git branch `main`. When it cannot tell which tests a change affects, it runs
every test. `nvs test --help` lists every flag, such as `--fail-fast`, `--rerun-failed` and
`--watch`.

The exit status is not zero when a test failed. A skipped test is not a failure.

**Good to know:** `--update` writes the new text into each failed `Core\Test::assertMatchesInline`
snapshot in the test's source file. Those tests are still reported as failed, so run the tests
again.
