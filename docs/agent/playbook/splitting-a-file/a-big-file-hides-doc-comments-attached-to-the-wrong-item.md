- **A big file hides doc comments attached to the wrong item.** Two of `nvs-types`' sat far from the
  function they described, invisible in a multi-thousand-line file and obvious the moment it became
  eight. When a carve leaves a doc block stranded above an unrelated item, that is a bug the split
  found, not one it made. [until: reviewed 2026-09-06]
