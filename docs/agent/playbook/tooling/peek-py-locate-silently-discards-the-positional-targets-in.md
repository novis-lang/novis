- **`peek.py --locate` silently discards the positional targets in the same call.** A call that
  read three regions and located one symbol answered with the anchor alone, so all three reads had
  to be sent again. Give `--locate` its own call, or drop it and read the regions whose anchors you
  already hold. [until: reviewed 2026-09-17]
