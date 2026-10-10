`nvs test main.nvs` compiles a program and runs every method that has the `#[Test]` attribute. It
prints one line for each test and a count at the end.

Tests can be in the same file as the code, or in a class of their own. The statements outside a
class do not run under `nvs test`.

`nvs test .` runs every `.nvs` file in a folder and its subfolders as one program. Run it in your
project folder, so every test can use the classes your `autoload` line loads. A `tests` folder
needs one file that requires your entry file.

`--filter <text>` runs only the tests whose name, `Class::method`, contains that text.
`--list` prints each test and its file, and runs none. `--format json` and `--format junit` print a
report for a build server. The exit status is `1` when a test failed, and `0` otherwise.

The examples show two tests, two test classes with a filter, and a report for a build server.
