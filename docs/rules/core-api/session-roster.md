The session surface is `start` plus the members that work on the record it loaded — read, write, remove,
clear, regenerate, destroy, and the sealed pair `rule:http-server/a-session-holds-a-secret-only-sealed`
adds. `start` is the only one that talks to the store: it is where a presented identifier is checked and
the record is loaded, and the rest operate on the record already in hand.

**A member called before `start` throws, naming the member that opens one.** That single rule is what a
per-request "this request uses sessions" declaration was buying — the fact is a line in the source — and it
is worth nothing if the first read can silently start one. One rule for all of them means they agree
rather than each growing a refusal of its own — the sealed pair answers it before it looks at a key ring
— and the throw is catchable at the root, so a program that cannot use sessions can say so.

`regenerate` issues a new identifier, moves the record to it and destroys the old entry, in that order, and
takes no argument: a flag to keep the old entry would choose between a fixation window and a lost session,
and only one of those is correct.
