The session surface is `start` plus six members — read, write, remove, clear, regenerate and destroy.
`start` is the only one that talks to the store: it is where a presented identifier is checked and the
record is loaded, and the six operate on the record already in hand.

**A member called before `start` throws, naming the member that opens one.** That single rule is what a
per-request "this request uses sessions" declaration was buying — the fact is a line in the source — and it
is worth nothing if the first read can silently start one. One rule for all of them means the six agree
rather than each growing a refusal of its own, and the throw is catchable at the root, so a program that
cannot use sessions can say so.

`regenerate` issues a new identifier, moves the record to it and destroys the old entry, in that order, and
takes no argument: PHP's delete-old-session flag chose between a fixation window and a lost session, and
only one of those is correct.
