An attack that breaks the member it was written against, and an example that disagrees with the
binary, are the program working. There are two answers and no third.

**Fix it**, in the slice that found it, with a case pinning the corrected behaviour. This is the
default, and most findings are small. **Record it**, when the fix is genuinely larger than a slice:
an entry in the owning crate's `# Known gaps` section, plus a marker on the proof naming that file.

A marked proof counts as a known gap rather than a failure, so a long unattended run is not stopped
by one bug it cannot fix. Two rules keep that from becoming a way to make anything green: the marker
must name a file that really carries such a section, so recording a bug means writing it where the
crate's own readers will find it; and **a marked proof that passes fails the sweep**, so removing
the marker is part of whatever fix eventually lands.

**Weakening the proof is not one of the two.** Softening the attack, re-blessing the example or
skipping the feature each turn a finding into a green check, which is the single outcome this rule
exists to prevent. A skip is for a proof that *cannot exist*, never for one that fails.
