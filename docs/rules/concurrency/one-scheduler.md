Concurrency is `Core\Task` and nothing else. Three members carry it — `Task::all` over a fixed set,
`Task::map` over a collection, and `Task::afterResponse` for work that follows a response — and
`spawn`/`await` and `Core\Task\Channel` are the primitives beneath them.

They run on one scheduler, which is the runtime's own: stackful coroutines, thread-per-core,
shared-nothing. **A second scheduler may not be linked into the binary.** The prior art is
`tokio::JoinSet` and `tokio::Semaphore`, and they are unusable here because they are built on a
future model this runtime does not have; a crate that brings an executor, a reactor or a `spawn`
with it brings a second concurrency model beside the coroutines, and the dependency graph is
checked for exactly that. Where `tokio` appears at all it is compiled with `sync` alone — no `rt`,
no `net`, no `time`, no executor — which makes it a channel library and not a runtime.

The guarantee the whole roster is built on is `rule:concurrency/nothing-is-still-running-when-a-call-returns`,
and it is a property of the call rather than of a scope object: there is no nursery, no task group
and no handle to leak, so a task tree is bounded by the same accounting a request already has.
