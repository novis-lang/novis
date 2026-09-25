- **A `Core\Db` hostile case that puts the `query` inside its loop attacks the driver rather than
  the member.** A million-round loop calling `$db->query(...)->all()` runs a million SQL statements:
  it did not finish in ten minutes under the debug binary, which reads as the member being
  unbounded, while hoisting the statement above the loop and calling `all()` a million times on the
  one result took 4.3s. Build the result set once above the loop, then attack the member that reads
  it. [until: gone crates/nvs-stdlib/src/db/mod.rs:Core\Db]
