`docs/examples/` is the only copy of every example, and the website build reads it in place. Nothing
copies it into `website/`, so there is no mirror to rebuild and no second copy to fall out of date.

They live in the repository because the same sweep that tests a feature writes its examples, and a
sweep cannot write into a tree it is not allowed to touch.

**Declined:**
- **Examples in the website's own tree, written by a person after the feature ships.** Written that
  way, almost no member ever had one. The sweep that understands the feature is the one that writes
  them.
- **A mirror of the examples under `website/`, kept equal by a sync script and a check.** Two copies of
  every example, and a failure mode, the mirror being behind, whose only fix is to run the script that
  should have run.
