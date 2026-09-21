Tells you why one step of a schema plan is there, in a sentence a person reads.

`Core\Db\Plan\Step::reason` returns one written sentence about this step: what it changes, what it
can cost, and why it is graded the way it is. A step that adds a nullable column says that every
existing row already has its value. A step that drops a table says that every row in it would go,
and that the plan reports the drop instead of running it.

The sentence is written by Novis, not by your program. It is meant for a person who has to approve a
release or read a deployment log. Print it, log it, or put it in the message a release tool shows.
Do not test what words it contains: a later version of Novis may explain the same step better.

Every step carries a reason, including the steps a plan will never run. Reading it changes nothing
and asks the database nothing, because the sentence was written when the plan was computed.

The examples read the reason of a single step, tell a person why a deployment stopped, and write the
release notes for whoever approves the change.
