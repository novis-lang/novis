- **`gaps.py --errors` matches a site by its message *before the first format hole*, so closing one
  member can silence its siblings.** A case echoing `Core\Time\DateTime::format(): …` contains the
  stem `Core\Time\DateTime::`, which is the whole stem of every message spelled
  `Core\Time\DateTime::{member}(…)`. The list is a worklist, not a ledger: when the drop is larger
  than the number of sites you asserted, diff it against the tree with your new cases moved aside
  and name the hidden ones in the handoff. [until: gone tools/gaps.py:--errors]
