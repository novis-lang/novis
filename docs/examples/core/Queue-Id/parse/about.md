Reads the string that `$id->toString()` gave and returns a `Core\Queue\Id` for the same job. A later
request can then call `Core\Queue::status` or `Core\Queue::cancel` for a job that an earlier request
added.

The string may be `tainted`, so a value from the URL can go in directly. A string that is not in the
form `toString` writes throws a `RuntimeError`. The message does not quote the string. When the
string may not be a job id, `Core\Queue\Id::tryParse` returns `null`.

The string has a random token. A string with a changed row number or token still parses, but finds
no job. `status` then throws a `RuntimeError`, and `cancel` returns `false`.

Any user who has the string can use the id. Save the string together with the user who started the
job. Look up both before you call `status` or `cancel`.

**The examples below** read an id back, show a wrong string and a changed one, and check the user in
a request.
