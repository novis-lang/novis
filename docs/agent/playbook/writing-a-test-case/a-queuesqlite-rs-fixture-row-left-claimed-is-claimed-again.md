- **A `queue_sqlite.rs` fixture row left `Claimed` is claimed again by the next `claim(…, NOW,
  NOW)`.** The claim's second arm is the visibility timeout — `state = 1 and claimed_at <= ?` — so a
  cutoff *at* the instant a fixture wrote `claimed_at` makes that row due beside the pending one, and
  the helper answers the older id while the case asserts the one it just pushed. Pass `NOW - WINDOW`
  as the cutoff in any case that leaves a claimed row behind, and keep `NOW` for the cases whose whole
  queue is pending. [until: reviewed 2026-09-10]
