`Core\Task::afterResponse()` runs a closure after the response is sent, so the visitor does not wait
for it.

Use it for work the visitor does not need to see: a receipt email, a webhook, an audit log. The call
only adds the closure to a list. The closures run when the request is finished, one at a time, in
the order you added them. In a command-line program, they run when the script ends.

**Good to know:** the work is not saved anywhere. If the request ends with an uncaught error or
`exit`, the closures do not run. If the process stops, the work is lost and nothing tries it again.
For work that must not be lost, use a real job queue. It replaces PHP's `fastcgi_finish_request`.

**The examples below** show a receipt sent after the response, several closures running in order,
and an audit log sent with a time limit.
