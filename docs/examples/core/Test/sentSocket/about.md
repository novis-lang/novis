`Core\Test::sentSocket()` returns every message your program sent over a fake WebSocket. You create
the fake service with `Core\Test::answerSocket`. The list is in the order the messages were sent,
and the first message comes first. Each item is a `Core\Socket\Message`.

For a text message, `text()` returns the text and `bytes()` returns `null`. For a binary message, it
is the other way round. The list contains the messages of every fake socket in the test, and reading
it does not clear it. Before your program sends anything, the list is empty.

**The examples below** show how to read the sent messages, how to count them, and a test of a chat
client that joins a room.
