- **A chain-switch failure in `.loop/log.md` numbers its check in the *folded* goal, not in the goal
  file you will open — and the reason may already be fixed.** `loop.py`'s `install_next` validates
  the live copy after `goal-switch.py` has folded the previous goal's floor checks in above the
  entry's own, so the index is offset by the floor count `goal-switch.py` prints on its first line.
  Run `python tools/chain.py --check`, which validates every queued entry, before spending anything
  on what the message says. [until: gone tools/goal-switch.py:goal-switch.py]
