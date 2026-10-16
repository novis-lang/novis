Returns the event stream your program is writing to.

An event stream is opened in one of two ways. A request calls `Core\Sse::stream` to answer with
events, or `Core\Sse::upgrade` to start a script that keeps sending after the request ends. Code that
did not open the stream calls `current()` to get it, and then calls `send()` on it. Every call
returns an object for the same stream, so events from different places arrive in the order they
were sent.

A program that has not opened a stream has none. In that case `current()` throws a `LogicError`.
This is also true for a response that writes an ordinary body over time with `Core\Response::stream`.

**The examples below** send an event from a helper, show the error when no stream is open, and
report the progress of an import job.
