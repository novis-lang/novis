The error a use of deprecated code throws when the setting `errors.deprecated` is `throw`. Code is
deprecated when it carries the attribute `#[Core\Deprecated]`. The error is a `LogicError`, because
the program itself still uses old code.

By default the setting is `ignore`, and deprecated code runs as usual. Set it to `throw` while you
test, and every use of old code stops with this error. Its message names the method or constant, the
version that deprecated it, and the code to use instead when the attribute names one.

A program can change the setting for its own request with `Core\Config::set('errors.deprecated',
'throw')`. The change applies to that request only. The next request starts with the setting from
`nvs.toml` again.

**The example below** shows a test run that finds a call to an old method and prints what to use
instead.
