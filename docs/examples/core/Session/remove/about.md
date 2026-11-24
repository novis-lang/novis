`Core\Session::remove()` deletes one key and its value from the session of the current request.
After it, `Core\Session::get()` returns `null` for that key. The other keys keep their values.
One `remove()` deletes a value of either lifetime, short-lived or long-lived.

Removing a key that the session does not have is allowed. It does not throw an error and does not
change the session.

You do not need to open the session first. The first call to any `Core\Session` method opens it.
The changed session is saved to the store when the request ends.

`remove()` throws a `LogicError` when the program is not handling a request, for example a
command-line program. It also throws a `LogicError` inside `Core\Html::later` and
`Core\Task::runAfterResponse`, because the response headers are already sent there. It throws a
`RuntimeError` when the session store cannot be opened, and an `IOError` when the store cannot be
reached.

**Good to know:** to delete many values at once, use `Core\Session::clearShortLived()`,
`Core\Session::clearLongLived()` or `Core\Session::clearAll()`.

**The examples below** show how to remove one value, what happens when the key is missing, and a
message that a page shows only once.
