`Core\Session::clearAll()` deletes the whole session of the current request from the store at once.
The response tells the browser to delete both session cookies.

After it, `Core\Session::get()` returns `null` for every key. You can save values again in the same
request. The first new value starts a new session with a new identifier.

A short-lived value, such as a sign-in, ends when the visitor closes the browser. A long-lived
value, such as a language choice, stays for 180 days by default. `clearAll()` deletes both kinds.
To sign a user out and keep their choices, use `Core\Session::clearShortLived()`.

`clearAll()` throws a `LogicError` in a program that is not answering a request. It also throws one
inside `Core\Html::later` and `Core\Task::afterResponse`, where most such calls do not compile. If
the store cannot be reached, it throws an `IOError`, and the data stays in the store.

**The examples below** show a session with both kinds of values deleted, a session used again after
`clearAll()`, and a page that deletes a visitor's data.
