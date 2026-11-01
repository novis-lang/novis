A `Core` handle is an object that carries a key into a table its task owns: a file, a socket, a
streamed reply or CSV walk, a database connection, a started script or a spawned process. A handle is
an ordinary value, so a `Core\Task` child task can return one. The key it carries is unique in the
process, drawn from one counter that every table of every task shares, so in the task that receives
it the key names nothing.

What a program sees there is the member's own refusal for a handle that names nothing. A file, a
socket and a script handle throw the error they throw once closed or awaited. A connection, a walk,
a streamed body and a process handle stop the program with a `FATAL` that says the handle is from
another task.

A key that was a position in its table broke this. The child's first file and the parent's first
file both had the key `1`, so a read on the child's handle read the parent's file. That reaches
another task's resource, which `rule:concurrency/a-child-belongs-to-the-calling-task` never grants.

- **Latency:** one atomic add per handle opened, and a binary search per handle lookup. A table's
  keys only grow, so the search needs no index.
- **Memory:** eight bytes of key per table entry, released with the task like the entry
  (`rule:programs/memory-priority`).

**Declined:**
- **A handle that works in every task of its request.** A child task's table goes with that task,
  so the handle would outlive what it names, and two tasks could race on one descriptor.
- **A task id in each handle beside a position key.** It is a second check every lookup must
  remember, where a unique key fails closed with no extra check at all.
