Two things run outside a request, and they are not variants of one mechanism. A `[[schedule]]` entry
is triggered by a clock, declared by the operator in root-owned configuration, carries no data, is not
retried, and a missed tick is simply missed. A queued job is triggered by a program calling `push`,
declared by the application, carries a payload, is durable until it succeeds or dead-letters, and is
retried within bounds.

"Every night at 03:00" is a schedule. "Because this request happened" is a job. A scheduled entry may
of course `push`, and that is the intended way to enqueue a nightly batch's worth of work: the clock
decides when the batch is created and the queue decides how each piece of it is run, retried and
recorded.
