- **A goal record's acceptance test can name *two* bounds, and splitting it across two `#[test]`s
  fails the check.** A `cargo-named` check matches the test's *name*, so
  `the_engine_floor_rotates_and_rate_limits_itself` split into `..._rotates` and `..._rate_limits`
  reads better, passes `cargo test`, and leaves the driver reporting `did not run` forever. Read the
  check's `tests` list before deciding how many functions the work becomes, even when it is two
  slices and two mechanisms. [until: gone tools/nv/cmd/orient.ts:const TRIAGE]
