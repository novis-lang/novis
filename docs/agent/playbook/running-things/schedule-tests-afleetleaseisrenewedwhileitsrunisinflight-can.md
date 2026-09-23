- **`schedule::tests::a_fleet_lease_is_renewed_while_its_run_is_in_flight` can fail one
  `python tools/verify.py` run and pass the next with nothing changed.** It asserts the lease was
  renewed *while* the run was still in flight, so the renewing task missing its slice reads exactly
  like a broken renewal, and `verify.py` runs the server's test binary beside every other one on
  purpose. Re-run `python tools/verify.py` once before reading it as a regression — twice red is a
  real one, and `verify.py`'s own message about a test that "passed alone" is the tell.
  [until: reviewed 2026-09-16]
