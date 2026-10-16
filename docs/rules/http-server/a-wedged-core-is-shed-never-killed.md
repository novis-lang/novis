What a watchdog report does is **report and shed, never kill**: the affected core stops accepting new work, and `max_in_flight` accounts for its share as unavailable so the remaining cores are not asked to carry a ceiling that assumes it (`rule:http-server/admission-is-arithmetic-not-a-number`). The watchdog reads the in-flight deadline each worker already keeps, so a worker pays nothing for being watched.

Admission subscribes to the watchdog's report and grows no detector of its own
(`crates/nvs-server/src/admit.rs`). Every sweep writes how many cores are watched, how many are wedged
and each core's own flag. A request on a reported core is refused a place, and the count refuses at
`max_in_flight` less the wedged cores' share, so one wedged core of two halves it and every core wedged
leaves nothing. The sweep that finds the core turning again clears both, and its share returns within
one interval. A request pays one thread-local read and three relaxed loads for it, and the process
holds two counts and one flag per core.

Killing is not available and the rule does not pretend otherwise. A thread cannot be safely killed in-process, and the process boundary that would make it possible is the one `rule:http-server/the-residue-is-one-named-fault-class` declines. The one process-level end this leaves is the service manager's own, and it is not this rule's kill: a
unit with `WatchdogSec=` set stops a process that has stopped pinging, and
`rule:packaging/the-generated-unit-is-hardened` gates that ping on this rule's detector — withheld only
where **no** core is turning, which is a process with nothing left to shed onto and nothing still
serving to be ended. Detection without a kill is still worth its cost: a wedged core that is reported degrades a service measurably, while a wedged core that is silent looks like a capacity problem for as long as anyone is willing to add capacity.
