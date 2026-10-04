`all` and `map` do not return while a child is still running. Every way out goes through the same
four steps — cancel the siblings, wait for those cancellations, collect, return:

| what happens | what the call does |
|---|---|
| every child returns | the shape (`all`) or the array (`map`) |
| one child throws | every sibling is cancelled, the call waits, and the **first** throw propagates |
| two children throw | the first by completion order propagates; the second is reported, never swallowed |
| the deadline expires | every child is cancelled, the call waits, `TimeoutError` is thrown |
| the calling task is cancelled | every child is cancelled and nothing is returned |

**`Core\Html::later` is the one call that returns with its task running**, because that task's
parent is the request and not the call (`rule:core-classes/html-later`). The guarantee holds one level
up: the response does not end while a `later` task runs, and the request's own teardown cancels and
waits for it exactly as the table above does for a child.

This is the guarantee the rest of the roster is built on. It is what makes a group inside a database
transaction safe to reason about: when the call returns, no child is still holding a row lock, and
nothing can write to a slot the code after the `catch` has already read.

It is bought at a stated price — `rule:concurrency/a-deadline-bounds-the-cancel-not-the-return` — and
it is why cancellation is the runtime's own teardown rather than anything a program participates in
(`rule:concurrency/cancellation-runs-no-user-code`).
