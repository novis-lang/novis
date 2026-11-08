Returns the job id as a string, so you can send it to a client and find the job again in a later
request. `Core\Queue\Id::parse` reads the string back into an id.

The string has three parts joined by dots: the job's number, a random token and the queue. It
contains only letters, digits, `.`, `_` and `-`. You can put it in a URL, a header, a cookie or JSON
without escaping. `echo`, string interpolation and `as string` give the same string.

The token is 16 random bytes. A string that a user changed or guessed finds no job: `status` throws a
`RuntimeError`, and `cancel` returns `false`. The id does not say who may use the job. Save the string
beside the user who started the job. In a later request, look it up with the signed-in user before
you call `status` or `cancel`.

**The examples below** put an id in a link, read a saved id back in a later request, and send an id
to the browser as JSON.
