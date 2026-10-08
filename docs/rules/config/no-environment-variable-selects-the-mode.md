**No environment variable is read for the mode. Not `NVS_MODE`, not `APP_ENV`, not `NODE_ENV`.**
No variable is ever populated by the host, and `Core\Env` is read-only; an env-var mode would be the
one place Novis reintroduced the exact mechanism behind every incident this design was shaped by —
Laravel's `APP_ENV=prod APP_DEBUG=true`, Django's settings dump on any 500, Node's `NODE_ENV` that
every library sniffs independently so a stack can be half in production with nothing detecting it.

Setting all three and asserting the mode is unchanged is a conformance case, not a convention. A
converted Laravel application's `APP_ENV` read arrives as an ordinary `Core\Env::get` and stays one:
it is that application's own variable, not Novis's mode.

The operator's runtime switch is saving the root-owned file, which the running server notices and
publishes as a whole new snapshot with no restart (`rule:config/the-config-is-an-immutable-snapshot`)
— strictly more capable than editing an environment variable, and root-owned.
