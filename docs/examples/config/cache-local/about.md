Sets how much the fastest cache tier may hold on each CPU core.

`Core\Cache::local()` hands back a store that belongs to one core and is shared with nothing — no
network, no locks — which is what makes it the fastest of the three tiers. `max_size` is the only
setting it has: what one core's entries may hold together, counting the keys as well as the values
they name. A machine running eight cores therefore holds up to eight times that figure, which is
what the setting is really buying.

**Good to know:** going over the limit never fails a write. The least recently used entries are
forgotten to make room, so a program plans for a read to miss rather than for a write to fail. And
the figure belongs to whoever runs the server: the memory it bounds is the core's, so a program that
could raise it would be spending what every other request on that core then goes without.
