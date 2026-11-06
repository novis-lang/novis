---
summary: The body of a streamed reply as server-sent events. `Core\Http\Stream::events` returns it, and a `foreach` loop reads one `Core\Http\Event` at a time
keywords: server-sent events, SSE, EventSource, text/event-stream, event stream, live updates, AI answer stream, streamed reply, Last-Event-ID
---

`Core\Http\Events` is the body of a streamed reply, read as server-sent events.
`Core\Http\Stream::events` returns it. Server-sent events are a text format for a series of
messages from a server, sent with the content type `text/event-stream`. You read it with a
`foreach` loop, and each item is a `Core\Http\Event`.

A server sends one event as a few `data`, `event` and `id` lines, and ends it with a blank line. The
loop gets the event when that blank line arrives. `data()` returns a `tainted string`, with the
`data` lines joined by a newline. `name()` returns the `event` line, or `null`. `id()` returns the
last `id` the server sent, and later events keep it until a new one arrives. A line that starts
with `:` is a comment and is skipped. Use `events()` for a `text/event-stream` reply, such as an AI
service that sends its answer word by word. For plain lines of text, use `Core\Http\Stream::lines`.

An event that the connection ends before its blank line is not given to the loop. One line may have
at most 64 KB, and the data of one event at most 1 MB. A longer one throws a `RuntimeError`. Text
that is not valid UTF-8 throws a `RuntimeError` too. A server that sends nothing for longer than
the call's `idle` time throws a `TimeoutError`.

The body of a stream can be read only once. After `events()`, a call to `events()`, `lines()`,
`chunks()` or `saveTo()` on the same stream throws a `LogicError`.

```nvs
<?nvs
// `Core\Test::answerHttp` gives a fake reply, so this example does not use the network.
// The first line is a comment. The second event has two `data` lines.
Core\Test::answerHttp('https://prices.example.com/live', 200, {
    body: ": connected\nevent: price\nid: 41\ndata: apples 1.20\n\nevent: notice\ndata: The market\ndata: closes soon.\n\n",
    headers: ['content-type' => 'text/event-stream'],
});

Core\Http\Stream $feed = Core\Http\Client::stream(Core\Http\Method::Get, 'https://prices.example.com/live');
foreach ($feed->events() as Core\Http\Event $event) {
    // The second event has no `id` line, so `id()` still returns 41.
    echo $event->name() ?? 'message', ' #', $event->id() ?? '-', ': ', $event->data(), "\n";
}
```
```output
price #41: apples 1.20
notice #41: The market
closes soon.
```
