`[deferred] deadline` is the time limit of work that runs after the response is sent.

`Core\Task::afterResponse` runs a function after the response has been sent. Sending an email or
writing an audit record is work of this kind. A call to `afterResponse` may give its own deadline.
A call that does not give one gets the value of this key. When this key is not written, such a
call has no deadline.

A request may change this key for itself with `Core\Config::set`. `[deferred] max_concurrent`
is in the same block, and only `nvs.toml` sets it. So a program can change its own deadline. It
cannot change how many requests may have work waiting at the same time.
