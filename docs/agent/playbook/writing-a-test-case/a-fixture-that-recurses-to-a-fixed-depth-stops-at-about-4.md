- **A fixture that recurses to a fixed depth stops at about 4,400 calls, not the "about 65,000
  frames" `rule:errors/stack-depth` names.** That figure is 8 MB of reserved stack over a minimal
  frame; an ordinary static method with one local throws `RecursionError` between 4,400 and 5,200
  calls, debug and release alike, and a fatter body stops sooner. Keep a proof that must succeed
  under about 2,000, and let one that must fail recurse with no base case at all.
  [until: reviewed 2026-09-19]
