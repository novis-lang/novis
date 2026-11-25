Returns the string that `Core\Queue\Id::parse` reads. `format` returns the same string as
`toString`. The string has three parts joined by dots: the job's number, a random token and the
name of the queue. The name is encoded, so `exports` is written `ZXhwb3J0cw`. The string contains
only letters, digits, `.`, `_` and `-`.

`Core\Queue\Id` implements the `Parses` interface, and `format` is the method that interface needs.
`Core\Router::url` calls it to write a job id into a link, in the path or in the query string.
`Core\Uri::buildQuery` calls it too. A URL never escapes the string, so the link contains it exactly.

The string works like a password for the job. Anybody who has it can ask for the state of the job.
Save the string beside the user who started the job, and check that user in a later request.

**The examples below** read the string back with `parse`, write an id into a link to a route, and
write one into a query string.
