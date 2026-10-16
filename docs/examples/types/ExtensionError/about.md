The error a call into an extension throws when the extension's code crashes. An extension is code
that runs in a sandbox beside your program, such as the built-in `Novis\Image` and `Novis\Intl`.
When that code stops on a bug of its own, your call throws `ExtensionError`. It is a `RuntimeError`,
so a `catch (RuntimeError $e)` catches it too.

The crash stays inside the sandbox. Your request goes on, and the next call to the same extension
works again. So you can catch the error and show the page without the part that failed.

An extension also reports ordinary problems, and those throw other classes. Bad input throws
`LogicError`, and text it could not read throws `ParseError`. A call that uses more time or memory
than the request may use is not an error you can catch: it ends the request.

**The example below** shows a page of uploads that keeps working when one thumbnail cannot be made.
