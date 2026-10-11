A wake handle dropped unused wakes the task it names, unless that task is the one dropping it. The far
side of a handoff that gave up, panicked or was torn down leaves its task parked on an answer that is
not coming, and the drop is the wake that turns that into a retry.

A task that drops its own handle is running, so it is parked on nothing. A wake queued at that moment
is collected only after the task has parked again, and it ends **that** wait instead: a task that takes
a handle for each wait in a loop would be woken by the handle of the wait before, and would never wait
at all. So the handle is collected on the spot. The reactor's outstanding count comes down on the core,
and nothing is queued or poked.

This is what lets a wait take a handle for itself and let it go when the wait ends for its own reason:
a drain wake held across one sleep, or the permission a drive gives back when it returns
(`rule:concurrency/one-permission-per-drive`). A wake that is **asked for** is queued whoever asks.

The drop checks both halves: the thread dropping the handle is the core that issued it, and the task
running there is the one the handle names. A task id is issued per scheduler and names a different
task on every core, so the task half alone is not enough. A reactor that is borrowed, gone or another
core's fails the check, and that drop delivers the wake as before. A registration made against a
drain that has already begun fires its wake before it returns, on the task's own stack, and a
stream's drain cut raises its flag the same way. Both are asked for, so both still leave a wake
queued for the task's next wait.

- **Memory:** nothing. The drop gains one branch, and no allocation is added per request, per task or
  per wait.

**Declined:**
- **Registering once, outside the loop that waits.** It fixes the one caller and leaves every later
  caller one loop away from a core kept busy by its own wakes.
- **A `cancel` the caller must choose to call.** Every site that gives up its own handle would have
  to remember it, and a registration on the drain holds its handle inside a closure that only a drop
  can reach.
- **Collecting every wake a task delivers to itself, the asked-for one too.** It would change what a
  registration against a drain that has already begun leaves behind, and the idle cost needs none of
  that changed.
