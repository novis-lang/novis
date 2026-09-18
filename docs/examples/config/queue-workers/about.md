How many queued jobs this instance works at once.

A deployment usually runs the same build on several instances, and this key is what makes them
different: some of them work the queue and some only add to it. Writing zero is a deployment rather
than a disabled queue — the instance enqueues jobs exactly as before and lets another one claim and
run them, which is how a web fleet hands its slow work to a pool sized for that work instead of for
requests.

**In plain words:** zero means "I add the work, somebody else does it". A queue is turned off by
leaving the block out altogether, not by hiring nobody.

The key belongs to whoever runs the deployment, and it is read once when the process starts. A
worker is a task the runtime spawns, so applying a new count means starting or stopping tasks rather
than reading a different number — an edited file and a reload name this key and carry the running
count forward until the next restart.

The example prints how many workers this instance runs and is turned away both hiring itself more of
them and sending its share of the work to everybody else.
