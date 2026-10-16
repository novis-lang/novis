A first push to a branch, a force-push and a shallow clone all leave the lane computation without a
base commit it can diff against. Every such case resolves to **every lane true**.

The failure mode of guessing wrong in the other direction is a merged commit nothing checked, and
there is no version of this that is worth one of those.
