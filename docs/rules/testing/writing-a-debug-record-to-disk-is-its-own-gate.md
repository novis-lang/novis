`[debug] store` decides whether debug records are persisted at all, is off by default, and can only
subtract — no run mode, no log level and no other directive turns it on.

It is deliberately not a reading of the run mode and deliberately not a level threshold. `[log] level`
is a legitimate operational dial whose production default is `Info` and which a tree may legally
raise to chase a bug; if the debug stream rode it, that ordinary action would silently begin
persisting complete typed dumps of request data. **A reasonable operational dial must not double as a
data-exposure switch.**

So this is a second gate read together with every ceiling already in force, and off wherever either
says off. `rule:testing/debug-mode-directive`'s reconnaissance argument does not weaken because a
second key said yes, and a tree that writes no `[debug]` block gets no debug stream at all.

With the gate off nothing reaches the disk, so a debug record that survives into a deployment costs
the predicted-not-taken branch `rule:testing/debug-probes` already pays and nothing further.
