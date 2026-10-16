Reads the body of a streamed reply as server-sent events, one event at a time.

Server-sent events are a text format for a stream of messages from a server. A server sends one
event as a few `data`, `event` and `id` lines, and ends it with a blank line. A `foreach` over
`events()` gives one `Core\Http\Event` for each finished event, as soon as it arrives. A line that
starts with `:` is a comment and is skipped.

The body of a stream can be read only once. After `events()`, a call to `events()`, `lines()`,
`chunks()` or `saveTo()` on the same stream throws a `LogicError`.

**Good to know:** an event that the server did not finish before the connection ended is not given
to your program. One line may have at most 64 KB, and one event's data at most 1 MB. A longer one
throws an error.

**The examples below** print the data of each event, handle events by their name, and collect the
text an AI service sends piece by piece.
