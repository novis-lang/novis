Removes the current connection from a topic, so values published to that topic no longer reach it.

Use it when a connection should stop listening to one topic but stay open. For example, a user
leaves one chat room and stays in the others. You do not need it when a connection ends: a
connection that closes leaves all its topics without any extra code. Leaving a topic the connection
never joined is not an error.

**Good to know:** only values published after the call stop arriving. A value that was already
waiting still arrives at the next `receive()`.

**The examples below** leave one of two topics, leave a topic that was never joined, and let a chat
user leave a room.
