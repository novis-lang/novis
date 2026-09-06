Waking sets an atomic flag and, if a permission is installed, delivers it. It does not decide the
future is ready, does not re-poll, and does not touch the run queue. **The re-poll belongs to the
loop**, and the loop clears the flag *before* it polls, so a wake fired from inside a poll is
observed by the check after it rather than by the iteration after that.

That ordering is the whole of the lost-wakeup argument, and it covers the two cases that would
otherwise be bugs.

A wake landing between the poll and the park finds no park to end and is the flag alone; the loop
re-reads the flag after arming its permission and before it suspends, so it goes round again instead
of sleeping on a wake that has already happened.

A waker cloned and held past the call fires into an empty slot and sets a flag nobody reads. That is
inert, and it is the same reading as a wake that raced with the task ending.
