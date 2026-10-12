What a watchdog report does is **report and shed, never kill**: the affected core stops accepting new work, and `max_in_flight` accounts for its share as unavailable so the remaining cores are not asked to carry a ceiling that assumes it (`rule:http-server/admission-is-arithmetic-not-a-number`). The watchdog reads the in-flight deadline each worker already keeps, so a worker pays nothing for being watched.

Killing is not available and the rule does not pretend otherwise. A thread cannot be safely killed in-process, and the process boundary that would make it possible is the one `rule:http-server/the-residue-is-one-named-fault-class` declines. The one process-level end this leaves is the service manager's own, and it is not this rule's kill: a
unit with `WatchdogSec=` set stops a process that has stopped pinging, and
`rule:packaging/the-generated-unit-is-hardened` gates that ping on this rule's detector — withheld only
where **no** core is turning, which is a process with nothing left to shed onto and nothing still
serving to be ended. Detection without a kill is still worth its cost: a wedged core that is reported degrades a service measurably, while a wedged core that is silent looks like a capacity problem for as long as anyone is willing to add capacity.

**Not on disk.** The detection is built: the watchdog reports a wedged core and never kills it
(`crates/nvs-host/src/watchdog.rs:28`), and the service manager's ping is withheld only where no core is
turning (`crates/nvs-cli/src/service.rs:1291`). The shedding is not: a wedged core keeps accepting, and
admission does not count its share of `max_in_flight` as unavailable.
