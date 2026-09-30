How many times a failing job is tried before Novis stops retrying it.

Some failures go away: a database was restarting, or a service was unreachable for a moment. Other
failures never go away. A job that fails is tried again, with a longer wait before each new try.
When it has been tried `[queue] max_attempts` times, Novis moves it to the dead-letter table and
stops. The job is not deleted. A person can read it there, fix the problem and put it back in the
queue.

The value must be at least 1, and there is no value that means "no limit". A server with `0` here
does not start.

Only the person who runs the server sets this key. A program cannot raise it or lower it. A change
applies to the next job that a worker takes, and the server does not need a restart.

The example prints the number of attempts and tries to change it. `Core\Config::set` returns
`false`.
