Reads a string as a job id, like `Core\Queue\Id::parse`, and returns `null` when the string is not
a job id. It never throws an error, so you can pass it a value from a URL or a form directly.

The string must have the form that `toString` writes. A string with the wrong parts, a character
that is not allowed, or too many characters gives `null`.

**Good to know:** `tryParse` checks the form only. A changed or guessed string in the correct form
still gives an id, but that id finds no job. `Core\Queue::cancel` returns `false` for it, and
`Core\Queue::status` throws an error. Your program decides who may use a job: save the string next
to the user who started the job, and look it up for the signed-in user first.

**The examples below** read a valid id, show which strings give `null`, and check an id from a
query parameter before the program cancels the job.
