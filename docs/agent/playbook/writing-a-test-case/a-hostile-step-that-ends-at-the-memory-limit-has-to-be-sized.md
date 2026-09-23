- **A hostile step that ends at the memory limit has to be sized to the ceiling, not merely above
  it.** A `finish` joining a hundred gibibytes spends the whole 60s timeout copying before the
  256 MiB ceiling fires, and `dossier.py --run hostile` then reports `unbounded, having printed 2
  line(s)`, which reads exactly like the attack finding a leak. Size the ending at single-digit
  gibibytes — two thousand pieces of a mebibyte reaches the ceiling in under a second — and leave
  the huge count on the step that proves the retention. [until: reviewed 2026-09-21]
