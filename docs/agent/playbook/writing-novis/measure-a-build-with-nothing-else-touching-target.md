- **Measure a build with nothing else touching `target/`.** The same narrowed release build timed
  twice as long with a `du -sh target` walking the tree beside it: on a link-heavy build the disk is
  the contended resource, so a second reader of the same tree doubles it. A timing run that
  disagrees with an earlier one by 2x is usually this and not the change under test.
  [until: reviewed 2026-09-06]
