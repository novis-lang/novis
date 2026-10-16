Sends the page to the browser as soon as the main script ends, and each slow part after it.

A page with `Core\Html::later` parts normally waits for every part before it is sent. Call
`Core\Response::slotted()` in the main script, and the page is sent first, with the placeholders
in it. Each part then follows as soon as it is ready, and the browser puts it in the place of its
placeholder. The visitor sees the page early, and the finished page is the same.

A route can do the same with `#[Core\Route(..., slotted: true)]`. Use the call when the decision
depends on the request, for example on its path. Calling it twice is the same as calling it once.
A page with no `later` part is sent whole.

**Good to know:** call it before the main script ends. Inside a callable that `later` or
`Core\Task::afterResponse` runs, it throws a `LogicError`. A failed part shows its `error` markup,
and the status stays the same.

**The examples below** show a page with two slow parts, a decision for one path only, and a part
that fails.
