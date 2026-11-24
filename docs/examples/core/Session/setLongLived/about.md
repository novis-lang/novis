`Core\Session::setLongLived()` saves a value under a key in the session. A later request from the
same visitor reads it with `Core\Session::get()`.

A long-lived value is for a choice the visitor made, such as their language or their cookie consent.
It is kept after the browser closes, for `[session] long_lived`, which is 180 days by default. Do not
save a login here. The visitor would stay signed in after closing the browser. For a login, use
`Core\Session::setShortLived()`.

A new value replaces the old value of the key, and the key becomes long-lived. The value can be a
string, a number, a bool, `null`, an array or an object.

The method throws a `LogicError` for a callable. It also throws a `LogicError` inside
`Core\Html::later` or `Core\Task::runAfterResponse`, and in a program that handles no request. It throws
a `RuntimeError` when the session store (the place where the server keeps session data) cannot be
opened. It throws an `IOError` when the store cannot be reached.

**The examples below** show how to remember a language, how to keep a shopping cart, and how a cookie
consent stays after a sign-out.
