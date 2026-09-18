Sets the two bounds on the cache tier that every core of one process shares.

Between the per-core store and the network-backed one sits a single store, held once for the whole
process. `max_size` is what its entries may hold together, and the same figure buys more here than
on the per-core tier because it is paid once rather than once per core. `fill_wait` is about misses
rather than size: when several requests want the same missing secret at the same moment, one of them
fetches it and the rest wait this long for that answer instead of all fetching their own. A request
still waiting when the time runs out gives up rather than piling onto the fetch.

**Good to know:** both settings belong to whoever runs the server. A wait of nothing would release
every queued caller empty-handed, so zero has no meaning here and the server refuses to start on one.
