`needsRehash` answers whether the stored hash is **weaker** than what hashing writes now, not whether
its parameters differ. A different algorithm is weaker by that rule's own terms, so a legacy hash
always answers `true`.

With the read roster that completes a migration with no flag, no tool and no second code path:
verification proves the password, this member says the row has fallen behind, hashing rewrites it, and
the legacy column converges to empty. A stored hash is not invertible, so a presented password at
login is the only point where a row can be rewritten, and this loop is the migration. It reads a legacy hash rather than throwing at it, which is the
one behaviour this member owes the roster.

A check that compares the stored parameters for difference answers `true` when a deployment
*lowers* its cost as readily as when it raises it. Measuring weakness rather than difference is the
direction that never rewrites a strong row into a weaker one.
