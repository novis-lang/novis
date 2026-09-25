- **A scratch `.nvs` file needs its `<?nvs` opener, and without one it "runs" and exits 0.**
  Everything before the tag is inline HTML, which lowers to an `echo` of the raw span, so the file
  prints its own source back and reports success — or, when that span is the whole file, dies in
  `nvs-ir`'s control-flow slice listing every statement it does lower, which reads as "`echo` is
  unsupported". Check that the output is the program's answer and not the program, or copy the first
  line from `examples/targets.nvs`; the `.nvst` harness supplies the tag inside `--FILE--`.
  [until: gone examples/targets.nvs:<?nvs]
