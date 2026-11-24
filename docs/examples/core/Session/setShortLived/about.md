`Core\Session::setShortLived()` saves a value under a key in the session. A later request from the
same visitor reads it with `Core\Session::get()`.

A short-lived value is for a login. It is deleted when the visitor closes the browser. It is also
deleted when the visitor sends no request for the time in `[session] short_lived`, which is 2 hours
by default. For a choice the visitor should keep, such as their language, use
`Core\Session::setLongLived()`.

A new value replaces the old value of the key, and the key becomes short-lived. The value can be a
string, a number, a bool, `null`, an array or an object.

The method throws a `LogicError` for a callable. It also throws a `LogicError` inside
`Core\Html::later` or `Core\Task::runAfterResponse`, and in a program that handles no request. It throws
a `RuntimeError` when the session store (the place where the server keeps session data) cannot be
opened. It throws an `IOError` when the store cannot be reached.

**The examples below** show how to save a value, how a new value replaces the old one, and how a
sign-out deletes a short-lived value.
