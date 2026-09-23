- **`echo "label=", <a call that may throw>` prints the label before it throws, so a `try` body
  leaks its prefix on the rows it exists to refuse.** The refusal's own text arrives welded to the
  label on one line. Bind the call first (`var $written = Core\Json::encode($v);`) and echo on the
  next line, so the refusing row prints nothing and the `--EXPECT--` block stays a list of the rows
  that answered; in the same family, a local obeys `E0112`, so `var $written_nan` is refused and
  `$writtenNan` is the spelling. [until: reviewed 2026-09-06]
