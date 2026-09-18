How many requests one core keeps alive at a time for work that runs after the answer has been sent.

Sending an invoice, warming a cache, writing an audit row: work like that is handed off so the
person waiting gets their answer first. The request it came from is not finished, though — it still
holds everything it was using — so a core that took such work on without a limit would run out of
memory under a burst of traffic rather than merely slow down.

The cap counts requests rather than pieces of work: one that hands off ten things is one of them.
Past it the next hand-off fails, in the request that asked for it, while that request is still
running and can do the work itself instead. Nothing is queued and nothing waits, because work
piling up out of sight is work that is quietly lost later.

Sizing a machine is not a program's decision, so a program may read this number and never raise it.
