`cancel` answers a `bool`, and the `bool` says whether *this* call is what took the job out of the
queue. A worker may claim a pending job at any moment, so a late cancel finding the work already
running is the ordinary outcome rather than an unlucky one, and answering `false` is how that is
reported. Throwing would make the commonest race an exception.

Cancelling changes the job's state rather than deleting its row, so a caller that cancels and then
asks `status` is answered `Cancelled` instead of being refused. The receipt still names something
`status` can answer about, which is what keeps the two members usable in the order a program actually
writes them (`rule:concurrency/queue-four-members`).
