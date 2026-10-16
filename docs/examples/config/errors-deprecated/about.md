What happens when a program uses deprecated code while it runs. Code is deprecated when it carries
the attribute `#[Core\Deprecated]`. You set it in `nvs.toml` as the key `deprecated` in the block `[errors]`.

There are three values:

- `ignore` runs the deprecated code as usual. This is the default.
- `log` runs the code and writes one warning to the log for each place a request uses it.
- `throw` throws `Core\DeprecatedError`. Its message names the method or constant and what to use
  instead.

The compiler also prints a warning for each use of deprecated code it finds. This setting is about
what happens when that code runs. A good use is `throw` in a test run, so every use of old code
stops the test.

A program can change the value for its own request with `Core\Config::set('errors.deprecated',
'throw')`. The next request starts with the value from `nvs.toml` again.

**The example below** runs a test with `throw`, finds a call to an old method, and then turns the
setting off for its own request.
