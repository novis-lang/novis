An attack that breaks the member it was written against, and an example that disagrees with the
binary, are the program working. There are two answers and no third.

**Fix it**, in the slice that found it, with a case pinning the corrected behaviour. This is the
default, and most findings are small. **Record it**, when the fix is genuinely larger than a slice:
a gap record under `data/gaps/` naming its owner, plus a `// proof: gap <gap id>` marker on the
proof naming that record.

A marked proof counts as a known gap rather than a failure, so a long unattended run is not stopped
by one bug it cannot fix. Two rules keep that from becoming a way to make anything green: the marker
must name a gap record that exists, so recording a bug means giving it an owner the owner gate holds
a goal to; and **a marked proof that passes fails the sweep**, so removing the marker is part of
whatever fix eventually lands.

**Weakening the proof is not one of the two.** Softening the attack, re-blessing the example or
skipping the feature each turn a finding into a green check, which is the single outcome this rule
exists to prevent. A skip is for a proof that *cannot exist*, never for one that fails.
