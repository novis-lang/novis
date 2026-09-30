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
