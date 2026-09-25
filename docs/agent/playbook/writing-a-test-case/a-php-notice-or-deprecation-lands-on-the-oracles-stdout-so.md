- **A PHP notice or deprecation lands on the oracle's *stdout*, so an `--ORACLE--` case that trips
  one can never match.** `hexdec("beefy")`, `base_convert("-255", 10, 16)` and `chr()` outside
  `0..255` each print a `Deprecated:` line before answering, and the rule is general: any input a
  twin *repairs* rather than refuses is a candidate. Run the oracle body through `php -r` while
  authoring — the notice is visible there and invisible in the `.nvst` diff — and split the repaired
  inputs into an `--ORACLE-DIVERGES--` file with a frozen `--EXPECT--`. [until: gone crates/nvs-test/src/case.rs:--ORACLE-DIVERGES--]
