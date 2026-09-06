A connection isolate holds the compiled unit it started with. An edit swaps the pointer for *new*
connections; connections already open run to completion on the code they began with.

This is the rule a long request already follows, applied to a longer-lived thing, and what it buys is
that a deploy does not break open connections — it drains them
(`rule:concurrency/a-drain-closes-a-connection-cleanly`). A connection whose code changed underneath
it would be resuming a loop whose statics, class table and function bodies had moved.

The swap is a write to the compilation table and nothing else: a program already handed out owns its
unit's pages, so the next resolve of that path hands the new unit to whoever asks next while the open
connection never sees it.
