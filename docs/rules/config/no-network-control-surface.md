There is no TCP listener, in either direction of configuration. `[control] socket` accepts a local
endpoint or `false`, and a value that would be reached over a network — a URL, a host and a port, a
bare port number — is refused at boot by name rather than bound. A remote control plane is reachable
today by running `nvs ctl` over the operator's existing access path, SSH or the container runtime's
exec, which every orchestrator already has.

A network listener is the only part of the control design that would carry an authentication
surface, and nothing yet needs one, so this is deferred rather than rejected. What it would take is
already recorded: a second listener absent unless configured and never sharing the application
listener; per-effect-class enablement (`observe` / `operate` / `lifecycle`) so a liveness probe cannot
be handed `shutdown`; a token read from a file, refused at boot if absent; TLS required for any
non-loopback bind; and `lifecycle` withheld from a network listener entirely.

A control endpoint on the application listener is rejected outright: every path-normalization bug
and proxy misconfiguration would become privilege escalation, and a reserved prefix would collide
permanently with the compile-time route table.
