- **A stem that has to reach `conformance_coverage.rs`'s corpus needs `Core\Test` with one
  backslash, which a Novis string literal will not give you.** The gate reads every case file as raw
  text and looks for the site's message stem literally; a `"Core\\Test::…"` written in the program
  lands as two backslashes and matches nothing, while an `--EXPECT--` line is plain text and carries
  it. Echo the message (or `Core\Str::slice` of its opening) and let the expectation hold the stem.
  [until: reviewed 2026-09-06]
