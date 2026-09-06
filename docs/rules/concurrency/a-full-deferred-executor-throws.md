Past `[deferred] max_concurrent`, `Core\Task::afterResponse` throws a `RuntimeError` **at the call
site**. It does not queue.

The throw arrives while the request is still running and can still decide what to do — respond
anyway, do the work inline, or tell the caller to retry — which is the only moment at which a
decision is available. The message names the directive an operator would change, because at this
point the program is not wrong; the deployment is loaded.

An unbounded queue in front of a non-durable executor is the worst of both designs: it hides the
overload and then loses the work anyway, and a bounded one is a durable queue with none of the
durability. A deployment reaching this cap has outgrown
`rule:concurrency/after-response-outlives-the-connection` and wants the durable queue it declines to
be. Failing loudly is what makes that legible, instead of receipts quietly ceasing to be sent.
