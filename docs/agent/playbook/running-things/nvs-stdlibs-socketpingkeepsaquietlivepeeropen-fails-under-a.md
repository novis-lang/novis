- **`nvs-stdlib`'s `socket_ping_keeps_a_quiet_live_peer_open` fails under a full `python
  tools/verify.py` and passes alone.** The case asserts that a quiet peer stays open, against a socket
  whose `idle` bound is 200 ms of wall clock, so the rest of the test binaries running beside it is
  enough to starve the ping and the `receive` throws `Timeout` instead. Re-run the gate once before
  believing that failure — `verify.py` tells you itself that the binary passed alone — and if it is
  the only red, it is not yours to fix inside an unrelated group.
  [until: reviewed 2026-09-17]
