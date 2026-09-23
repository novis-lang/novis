- **The handoff's named group may already be on disk under a filename that does not say so, and
  `gaps.py` will not tell you.** `gaps.py` counts *cases* per member, so a member three sweeps
  mention in passing still ranks thin while the property is already pinned. Before reading the
  implementation, `ls tests/conformance/core/ | grep -i <class>`, `sed -n '2p'` over every hit and
  read the case *bodies*; if the item is already answered, say so in the handoff.
  [until: reviewed 2026-09-06]
