Tells you what has happened to a background job: whether it is waiting, running, done, failed or cancelled.

You pass the id that `Core\Queue::push` returned, and `status` returns one of the five `Core\Queue\State` cases. It reads the state from the database on every call, so the answer is always the current one. A job that failed once is retried, and stays `Pending` until it runs again. It becomes `Dead` only after it used all its attempts.

**Good to know:** `Succeeded` means the job ran, but a job can run more than once. If `delete` removed the job, `status` throws an error for it.

**The examples below** read the state of a new job, show a cancelled job, and turn each state into a message for the user.
