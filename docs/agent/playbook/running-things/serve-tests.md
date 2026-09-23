- **`serve::tests::the_in_flight_ceiling_is_fleet_wide_so_a_hot_core_cannot_refuse_while_neighbours_idle`
  fails under load and passes alone**, and `verify.py`'s message for it names a shared port, path or
  container, none of which it has. It drives a fleet-wide semaphore across threads, so a loaded machine
  lets one core's handler run after the ceiling already refused it — a session whose diff is nowhere near
  `crates/nvs-server` can spend calls proving the failure is not its own. Re-run `python tools/verify.py`
  and only investigate if two consecutive runs name it. [until: reviewed 2026-09-12]
