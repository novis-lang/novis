The reactor reports **readiness**, never completion, on every platform: epoll on Linux, kqueue on the
BSDs and macOS, and on Windows a poll of `\Device\Afd` through the completion port, which turns it
into a readiness source. One contract, three back ends, and the stream code above it is written once.

That choice is the reason there is one stream implementation rather than two. Completion semantics are
a *different* contract — a completion-based read owns its buffer across a suspension and a
readiness-based one does not — so adopting the native Windows shape would duplicate every stream and
every helper that reads one, on the platform that gets the least traffic.

The readiness layer is a poller and not an async runtime: no executor, no task type, no futures. The
runtime brings its own scheduler (`rule:concurrency/the-parking-contract`), and nothing
attacker-controlled reaches a registration, which carries a descriptor and an interest set and no
parsed input at all.
