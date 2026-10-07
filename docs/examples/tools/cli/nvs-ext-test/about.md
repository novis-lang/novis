`nvs ext test` runs the tests of an extension project with the built `.nvsx` file loaded.

The command runs every method that has `#[Test]` in the `.nvs` files of the project's `tests`
folder, as `nvs test` does. The tests can call the class of the extension. Run `nvs ext build`
first: if a file of the project changed after the last build, the command stops with an error.

The command does not read `./nvs.toml`. So in a test the extension may not read files or connect to
hosts. To test with grants, name a configuration with `--config`.

**Good to know:** if the configuration you name lists the `.nvsx` file with another `sha256`, the
command stops with an error that shows both values.
