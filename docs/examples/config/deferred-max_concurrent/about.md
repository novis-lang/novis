How many requests one core keeps open at a time for work that runs after the response is sent.

Sending an invoice, filling a cache or writing an audit row can run after the response. The person
waiting then gets the response first. The request is not finished yet, and it keeps all the memory it
was using. Without a limit, many such requests at once could use all the memory of a core.

The limit counts requests, not pieces of work. A request that adds ten functions counts once. When
the limit is reached, `Core\Task::runAfterResponse` throws an error in the request that called it.
That request is still running, so it can do the work itself. Nothing is put in a queue.

Only `nvs.toml` sets this value. A program may read it, but it cannot change it.
