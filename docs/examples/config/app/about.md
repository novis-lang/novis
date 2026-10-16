Configures one application: which settings, limits and permissions apply to a program, chosen by
where that program's file lives.

One server can run several applications side by side. An `[[app]]` block names either a directory or
a single entry file, and everything written inside it applies only to programs under that name.
Where more than one block covers a program, the more specific block wins, and a block that says
nothing about a setting leaves whatever the wider one said in place. A block can take permissions
away as well as grant them, and narrow limits as well as widen them, but it can never reach past the
ceilings set for the server as a whole.

**Good to know:** these blocks belong to whoever deploys the server. A running program can read the
settings it ended up with, and can tighten its own limits for the rest of its run, but it can never
change the block that configured it — one that could would be choosing its own permissions, or
another application's.
