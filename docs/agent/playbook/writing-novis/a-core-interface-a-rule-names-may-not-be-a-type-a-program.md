- **A `Core` interface a rule names may not be a type a program can write, and nothing says so at
  the declaration.** `Core\Db\Queryable` is `rule:core-classes/db-transactions`'s "a connection or a
  transaction", and it is in no section of `nvs meta --json`, so a parameter declared with it
  compiles and then fails at every use — `E0405` on a method call, `E0401` on both classes as
  arguments. An unknown class name in a parameter type is accepted silently, so read `nvs meta
  --json`'s `interfaces` list before typing a parameter with a `Core` interface.
  [until: gone crates/nvs-stdlib/src/db/mod.rs:Core\Db\Queryable]
