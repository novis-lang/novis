Returns the cookie of a sync, which you give to the next call of `changes`.

A sync with `changes` returns the entries that changed since the last run. The cookie is how the
server knows when the last run was. Read it after the loop, and store it, for example in a database.
The next run gives it back with the `cookie` option and gets only the entries that changed since.
The first run has no cookie, so it returns every matching entry.

The cookie is bytes. To store it as text, convert it with `Core\Encoding::toBase64`.

**Good to know:** if the loop stops early, the cookie covers only the entries it read. The next run
returns the other entries again, so nothing is lost. The cookie is `tainted`, because the server
sent it.
