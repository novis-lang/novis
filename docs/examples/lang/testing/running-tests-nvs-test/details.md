`nvs test tests/` runs every `.nvs` file in `tests/` and its subfolders as one program. One file
there requires the entry file of your application, and its name can be anything. The statements
outside the classes do not run, in any of the files.

A `.nvst` file, or a folder of `.nvst` files, is a different kind of test. Each file has a
`--TEST--` title, a program under `--FILE--` and its expected output under `--EXPECT--`. The report
is one line, such as `1 passed, 0 failed, 0 skipped`.

A folder with both `.nvs` and `.nvst` files is an error. `nvs test` takes one program at a time, so
a `.nvs` path and a `.nvst` path in one call are an error too. `--format`, `--list` and `--update`
do not work with `.nvst` files.

`--format json` prints one JSON document at the end. It has `schemaVersion` `2`, a `summary` with
`total`, `passed`, `failed`, `skipped`, `flaky` and `durationMs`, and a `tests` list. Each test has
`class`, `method`, `file`, `line`, `column`, `verdict` and `durationMs`. `line` and `column` are
where the method's name is written, counted from `1`. A skipped test also has `reason`. A failed or
flaky test has `failures`, and a flaky test has `attempts`.

`--coverage-lcov`, `--coverage-clover` and `--coverage-cobertura` write which lines the tests ran.
With one of them, the JSON document has `schemaVersion` `3`, and each test has `coverage`. It maps
each file name to the lines that test ran.

`--format junit` prints JUnit XML, with one `<testsuite>` for each class and one `<testcase>` for
each test. A test has `<skipped>`, `<failure>` or `<flakyFailure>` inside it. The `message` of a
failure is its first message, and the text inside is every message.

With `--format json` or `--format junit`, standard output has only the document. The output of a
test that fails goes to standard error.

`--list` prints each test and where it is written, such as `T::a  main.nvs:6`, and runs nothing. A
test with data rows has one line for each row. With `--format json`, it prints a document with
`schemaVersion` `2` and a `listed` list of `class`, `method`, `file`, `line` and `column`.

`--filter` is case-sensitive, so `--filter t::` does not select the class `T`. A data row
is `Class::method#0`, `Class::method#1` and so on. A class with no selected test is not printed, and
its fixtures are not built.

The exit status is `1` when a test failed, and `0` otherwise. It is the same in every format.
