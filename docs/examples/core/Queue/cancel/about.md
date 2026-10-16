Stops a background job before it starts.

You pass the id that `Core\Queue::push` returned. If the job is still waiting, `cancel` marks it as cancelled and returns `true`. No worker will run it. If a worker already started the job, or the job already ran, `cancel` returns `false` and changes nothing. A job that is running is never stopped.

The row of a cancelled job stays in the database, so `status` returns `Cancelled` for it. `delete` removes the row.

**The examples below** cancel a waiting job, show what a second `cancel` returns, and cancel the reminders of a user who unsubscribed.
