A job names a script file — not a class, not a callable, not a static method. It is the third
construct to take that shape, after `spawn script` and a connection upgrade, and the reason it takes
the *narrowest* of the three is the row: a job's target is stored as data and claimed by any host in
the fleet, possibly after a redeploy, and a string in a table can hold a path but not a method
reference.

A payload crossing into the job is a value **copied**, never a reference, and it is decoded on the
other side into declared types through the derived codecs. That makes a job a compiled unit like any
other file — cached, hot-reloadable, traceable and coverable with no special case
(`rule:concurrency/a-job-runs-as-a-root-isolate`) — and it is why reconstructing an object from a
payload is a question the runtime never has to answer.

`spawn script` and a connection upgrade also accept a static method, and a job does not, for the row
reason above. Three constructs that run code sharing one shape is worth more than any one of them, so
a fourth such construct takes the same shape or says why not.

**Declined:**
- **A job as a class implementing an interface**, discovered by enumeration. It reads better, and it is
  what a framework layer can give on top. As the `Core` shape it would need a rule for rebuilding an
  instance from a payload, where the derived codecs already decode plain data, and a file is what the
  isolate machinery already takes.
