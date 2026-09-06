`crates/nvs-db` is one workspace crate below `nvs-stdlib`, depending on the runtime, the host's
stream and TLS, and configuration. `nvs-stdlib` depends on it and never the reverse: `Core\Db`'s
registry rows and reference cards stay where every other class's are, and the wire lives here, so the
graph stays a tree. A crate rather than a module, because five protocol implementations would
otherwise be rebuilt by every session that only touched `Core\Str`, and because the dependency set an
audit most wants to look at is worth having behind one manifest.

**No per-driver feature flags.** Every driver is in every binary, because `Core` means always
present and because the same binary should behave the same way everywhere — a feature matrix makes
"does this deployment speak MariaDB" a property of how someone built it. The cost is compile time and
binary size, spent to buy simplicity.

The rule the driver table encodes: **a codec is borrowed, a state machine is written.** Message
framing, value encoding and authentication mechanisms are large, fiddly and identical for everyone,
which is exactly what a sans-IO crate is. Sequencing — what to send next, what a park in the middle
means, when the connection is reusable — is where this project's own decisions live, and borrowing it
is what would have dragged an async runtime in. TDS has no such crate and is written by hand.
