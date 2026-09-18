What has become of a background job. Asking the queue about a job you handed it earlier answers with
one of five states, and between them they cover everything a job can be doing.

`Pending` means it is waiting for a worker to pick it up — including while its start time is still in
the future, and while it waits between attempts. `Claimed` means a worker is holding it and running
it right now. `Succeeded` means it ran to the end. `Dead` means it used up every attempt it was
allowed and has been set aside, with its payload and each attempt's error, where it stays until a
person deals with it. `Cancelled` means it was called off before any worker took it.

**Good to know:** there is no `Failed` state, because a failed attempt does not end a job. It sends
the job back to `Pending` to be tried again, and only running out of attempts makes it `Dead`. So the
one state that means work somebody asked for never happened is `Dead`, and that is the one worth
watching.
