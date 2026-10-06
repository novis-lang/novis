Changes one setting for this request.

The change belongs to the request that makes it. No other request sees it, and it is gone when the
request ends. The name is read the way `Core\Config::get` reads it, and the value is written the way
a configuration file writes it: `512M`, `30s`, `64`.

The result is `true` when the change was made. It is `false`, with the old value still in force, when
the change is not allowed: a setting only the server may set, a name that is no setting at all, a
value that is not written the way the name needs, and a value above the ceiling the server keeps for
that name. A refused change throws no error, so a program that needs to know reads the result.

**Good to know:** `Core\Config::restore` puts the configuration file's own value back.
