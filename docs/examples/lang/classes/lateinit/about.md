A `lateinit` property can get its value the first time a program reads it. The class implements the
interface `LateInit` and writes one method, `initProperty`.

When a read finds a `lateinit` property with no value, Novis calls `initProperty` with the name of the
property. The method sets the property, and then the read returns the new value. A later read does not
call the method again. A write to one element, such as `$post->tags[] = "news"`, calls the method first
too.

If `initProperty` returns and the property still has no value, the read throws a `LogicError`. If the
method throws an error, the property still has no value, and the next read calls the method again.

**Good to know:** the write that gives the property its first value inside `initProperty` does not run
a `set` hook, `PropertyGuard` or `PropertyObserver`. A second task that reads the property while the
method runs waits until it returns.

**The examples below** show a property set on the first read, one method for several properties, a
second try after an error, and the error when a property stays empty.
