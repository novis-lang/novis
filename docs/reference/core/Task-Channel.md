---
summary: a bounded queue between two tasks whose `send` waits at the bound — backpressure instead of a growing buffer
keywords: channel, backpressure, bounded queue, producer, consumer, pipeline, streaming between tasks, close, capacity
---

`Core\Task\Channel<T>` carries values from one task to another. It is built with
`new Core\Task\Channel<T>($capacity)` — the capacity is at least `1` — and holds that many values: a `send`
into a full channel suspends the sending task until the consumer has taken one, which is the whole of the
backpressure. The channel is its own consumer loop: a `foreach` over it takes each value out, waits while
the channel is empty and still open, and ends once the channel is empty and closed. `close` is what ends
that loop, and every value sent before it is still delivered; a `send` after `close` is a fatal error that
no `catch` sees. Both ends are normally children of one `Core\Task::all`, so the call returns only when the
producer and the consumer are both done.

```nvs
<?nvs
class Log {
    public static string $seen = "";
}

var $line = new Core\Task\Channel<int>(2);

Core\Task::all({
    produce: fn(): int => {
        foreach (Core\Arr::range(1, 6) as int $n) {
            $line->send($n * $n);
        }
        $line->close();
        return 6;
    },
    consume: fn(): int => {
        foreach ($line as int $square) {
            Log::$seen = Log::$seen . $square . " ";
        }
        return 0;
    },
});

echo Log::$seen, "\n";
```
```output
1 4 9 16 25 36
```
