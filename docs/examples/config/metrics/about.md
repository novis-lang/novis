Where a deployment's numbers go, and how many of them one core keeps.

Novis counts what it does — requests served, time and memory spent, failures reported — and this
block says who collects those counts. The exporter is off unless something is written; otherwise it
names a scrape endpoint the engine answers at the address beside it, or a collector it pushes to
instead. One more key bounds how many distinct series a single core will hold, so a label built out
of something a visitor sent cannot grow the registry without end.

The whole block is the operator's. A program able to switch the exporter off, or to point it
somewhere else, could stop being measured at the moment being measured matters most.

A value naming no protocol is refused when the configuration file is read, because what it produces
otherwise is silence — and a collector nothing ever writes to looks exactly like a deployment with
nothing to say.

The example prints what this checkout exports and asks for each of the keys in turn.
