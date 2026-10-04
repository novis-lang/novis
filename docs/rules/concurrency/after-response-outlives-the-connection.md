`Core\Task::afterResponse(callable $fn, {deadline?}): void` registers work to run once the request
is done with:

```php
Task::afterResponse(fn(): void => Receipts::send($order), {deadline: 30s});
return $response;         // the client has its bytes; the receipt is still going out
```

The trigger is **the response being complete**: the request task's own frame has returned and every
task `Core\Html::later` started has ended (`rule:core-classes/html-later`). A request with no `later`
call is complete when its frame returns. Under a server that is the moment the response is fully
written, slots and all; under `nvs run`, which has no response at all, it is the end of the script.
One rule, and the member means the same thing on every host. A request that ended by a throw, an
`exit` or a `FATAL` runs none of it: that status is what the host is about to report, and script
running over it would lose one of the two.

**Memory, CPU and tasks stay charged to the request tree**, which is why this is affordable at all —
the tree simply stays in flight a little longer than the connection does, and every limit but
`wall_time` still bounds it. The connection is detached from the tree before the work runs, so the
peer going away cancels nothing.

**It is not a queue.** Nothing is durable, nothing retries, and a process that dies loses the work
with no record. Receipts, webhooks, cache warming and audit shipping are what it is for; anything
that *must* happen belongs in the transaction that made it necessary or in a store the application
owns.
