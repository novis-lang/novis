The reactor reports **readiness**, never completion, on every platform: epoll on Linux, kqueue on the
BSDs and macOS, and on Windows a poll of `\Device\Afd` through the completion port, which turns it
into a readiness source. One contract, three back ends, and the stream code above it is written once.

That choice is the reason there is one stream implementation rather than two.

The readiness layer is a poller and not an async runtime: no executor, no task type, no futures. The
runtime brings its own scheduler (`rule:concurrency/the-parking-contract`), and nothing
attacker-controlled reaches a registration, which carries a descriptor and an interest set and no
parsed input at all.

**Declined:**
- **Completion semantics on Windows.** It is the native contract there and faster at very high
  connection counts, but a completion-based read owns its buffer across a suspension and a
  readiness-based one does not. Every stream and every helper that reads one would exist twice, and
  the copy that gets less traffic would be on a first-class platform.
- **Hand-written epoll, kqueue and AFD back ends.** Three `unsafe` FFI layers with nothing specific to
  this runtime in any of them, against a pure-Rust poller whose whole surface is exactly this. What is
  specific is the parking contract above it.
- **An async runtime such as `tokio`.** Its value is an executor and a task model, and the runtime has
  its own, in which a stackful coroutine suspends from *inside a helper*, which a future cannot.
